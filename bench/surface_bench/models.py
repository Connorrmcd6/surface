"""Provider-agnostic model adapters.

Every model implements `complete(system, user) -> Completion`. Anthropic is the first provider;
adding OpenAI/Gemini/etc. means writing one more class behind the same protocol and registering it
in `build_model`. Nothing else in the harness knows which provider produced a completion.

`Completion` carries token usage. We compare *output* tokens across conditions: input tokens differ
by construction (the doc block's size), so they are structural, not a behavioural signal; output
tokens are where the cost of reconciling a stale doc against the code actually shows up.
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass, field
from typing import Any, Protocol


@dataclass
class Completion:
    text: str
    input_tokens: int = 0
    output_tokens: int = 0
    raw_usage: dict = field(default_factory=dict)


class Model(Protocol):
    name: str

    def complete(self, system: str, user: str) -> Completion: ...


# ---- Multi-turn (agentic) types ------------------------------------------------------------
# A provider-neutral one-turn result. Each adapter (Anthropic/OpenAI/Gemini) translates its wire
# response into a `Step` and stashes the raw assistant message in `provider_msg`, so the agent loop
# can echo it back verbatim on the next turn without the loop ever learning the provider's shape.


@dataclass
class ToolCall:
    id: str
    name: str
    args: dict


@dataclass
class Step:
    text: str = ""
    tool_calls: list[ToolCall] = field(default_factory=list)
    input_tokens: int = 0
    output_tokens: int = 0
    stop_reason: str = ""
    provider_msg: Any = None  # raw assistant message, re-sent verbatim into history


class ToolModel(Protocol):
    """A model that can run the multi-turn loop. Separate from `Model` so single-shot-only adapters
    don't have to implement `step`. `messages` is the neutral history built by `agent.run_agent`."""

    name: str

    def step(self, system: str, messages: list[dict], tools: list[dict]) -> Step: ...


class MockModel:
    """Offline model for pipeline tests. Returns a canned reply (optionally per-condition).

    `replies` maps a condition label -> reply text; `default` is used otherwise. This lets a
    dry run exercise grading, metrics, and reporting with no network or API key.
    """

    def __init__(self, name: str = "mock", default: str = "", replies: dict | None = None):
        self.name = name
        self._default = default
        self._replies = replies or {}
        self._condition: str | None = None

    def set_condition(self, condition: str) -> None:
        self._condition = condition

    def complete(self, system: str, user: str) -> Completion:
        text = self._replies.get(self._condition, self._default)
        # Synthetic output-token count so metrics/report have something to aggregate offline.
        return Completion(text=text, input_tokens=len(user.split()), output_tokens=len(text.split()))


class MockToolModel:
    """Offline tool-using model for loop tests: returns a fixed `script` of `Step`s, one per call.

    Once the script is exhausted it falls back to a text-only `Step` (no tool calls), which the loop
    treats as a final answer — so a script that never calls `final_answer` still terminates cleanly
    (exercising the max-turns / forced-answer path). No network, no key.
    """

    def __init__(
        self,
        name: str = "mock-tool",
        script: list[Step] | None = None,
        fallback: str = "",
        default: str = "",
        replies: dict | None = None,
    ):
        self.name = name
        self._script = list(script or [])
        self._fallback = fallback
        self._default = default
        self._replies = replies or {}
        self._condition: str | None = None
        self._i = 0

    def set_condition(self, condition: str) -> None:
        self._condition = condition

    def step(self, system: str, messages: list[dict], tools: list[dict]) -> Step:
        if self._i < len(self._script):
            step = self._script[self._i]
            self._i += 1
            return step
        if self._fallback:
            # Text-only turn -> the loop accepts it as the answer (exercises the forced-answer path).
            return Step(text=self._fallback, output_tokens=len(self._fallback.split()))
        # Canned mode (run.py offline smoke): answer immediately with the condition's reply.
        reply = self._replies.get(self._condition, self._default)
        return Step(
            tool_calls=[ToolCall(id="final", name="final_answer", args={"answer": reply})],
            output_tokens=len(reply.split()),
        )


# ---- Anthropic tool-use translation ---------------------------------------------------------
# Pure converters between the neutral loop format (agent.run_agent) and the Anthropic wire format,
# kept at module scope so they can be unit-tested without a network call (the riskiest part of any
# provider adapter is the message/tool round-trip).


def _anthropic_tools(tools: list[dict]) -> list[dict]:
    return [
        {"name": t["name"], "description": t["description"], "input_schema": t["parameters"]}
        for t in tools
    ]


def _anthropic_blocks_from_step(step: Step) -> list[dict]:
    # Fallback reconstruction when a Step has no provider_msg (e.g. a mock); the real adapter always
    # stores provider_msg, so this just keeps history valid for non-Anthropic-authored turns.
    blocks: list[dict] = []
    if step.text:
        blocks.append({"type": "text", "text": step.text})
    for tc in step.tool_calls:
        blocks.append({"type": "tool_use", "id": tc.id, "name": tc.name, "input": tc.args})
    return blocks


def _anthropic_messages(messages: list[dict]) -> list[dict]:
    # Anthropic requires alternating roles, so we coalesce consecutive same-role turns — in
    # particular a tool_result user-turn followed by a nudge user-turn become one user message.
    out: list[dict] = []

    def push(role: str, blocks: list[dict]) -> None:
        if out and out[-1]["role"] == role:
            out[-1]["content"].extend(blocks)
        else:
            out.append({"role": role, "content": list(blocks)})

    for m in messages:
        if m["role"] == "user":
            push("user", [{"type": "text", "text": m["content"]}])
        elif m["role"] == "assistant":
            step = m["step"]
            blocks = step.provider_msg if step.provider_msg is not None else _anthropic_blocks_from_step(step)
            push("assistant", blocks)
        elif m["role"] == "tool":
            push(
                "user",
                [
                    {"type": "tool_result", "tool_use_id": r["id"], "content": r["content"]}
                    for r in m["results"]
                ],
            )
    return out


def _step_from_anthropic(resp) -> Step:
    text = ""
    calls: list[ToolCall] = []
    provider: list[dict] = []
    for b in resp.content:
        btype = getattr(b, "type", None)
        if btype == "text":
            text += b.text
            provider.append({"type": "text", "text": b.text})
        elif btype == "tool_use":
            args = dict(b.input)
            calls.append(ToolCall(id=b.id, name=b.name, args=args))
            provider.append({"type": "tool_use", "id": b.id, "name": b.name, "input": args})
    u = resp.usage
    return Step(
        text=text,
        tool_calls=calls,
        input_tokens=getattr(u, "input_tokens", 0),
        output_tokens=getattr(u, "output_tokens", 0),
        stop_reason=getattr(resp, "stop_reason", "") or "",
        provider_msg=provider,
    )


class AnthropicModel:
    def __init__(self, name: str, model_id: str, temperature: float, max_tokens: int):
        try:
            import anthropic
        except ImportError as e:  # pragma: no cover
            raise SystemExit("pip install anthropic (see bench/pyproject.toml)") from e
        if not os.environ.get("ANTHROPIC_API_KEY"):
            raise SystemExit("ANTHROPIC_API_KEY is not set")
        self.name = name
        self.model_id = model_id
        self.temperature = temperature
        self.max_tokens = max_tokens
        # Per-request timeout + retries so a single hung request can't stall the whole matrix
        # (the SDK default has no wall-clock cap short enough for a long unattended run).
        self._client = anthropic.Anthropic(timeout=120.0, max_retries=4)

    def complete(self, system: str, user: str) -> Completion:
        resp = self._client.messages.create(
            model=self.model_id,
            system=system,
            max_tokens=self.max_tokens,
            temperature=self.temperature,
            messages=[{"role": "user", "content": user}],
        )
        text = "".join(b.text for b in resp.content if getattr(b, "type", None) == "text")
        u = resp.usage
        return Completion(
            text=text,
            input_tokens=getattr(u, "input_tokens", 0),
            output_tokens=getattr(u, "output_tokens", 0),
            raw_usage=u.model_dump() if hasattr(u, "model_dump") else {},
        )

    def step(self, system: str, messages: list[dict], tools: list[dict]) -> Step:
        resp = self._client.messages.create(
            model=self.model_id,
            system=system,
            max_tokens=self.max_tokens,
            temperature=self.temperature,
            tools=_anthropic_tools(tools),
            messages=_anthropic_messages(messages),
        )
        return _step_from_anthropic(resp)


# ---- OpenAI tool-use translation ------------------------------------------------------------
# Same pattern as the Anthropic block: pure converters between the neutral loop format and the
# OpenAI Chat Completions wire format, unit-tested without a network call. NOTE for smoke time:
# newer GPT models may need `max_completion_tokens` instead of `max_tokens` and may reject a custom
# `temperature` — adjust in OpenAIModel.step when the exact model id is pinned.


def _json_args(raw: str) -> dict:
    try:
        return json.loads(raw) if raw else {}
    except json.JSONDecodeError:
        return {}  # a malformed tool call becomes empty args; the loop feeds back an error and recovers


def _openai_tools(tools: list[dict]) -> list[dict]:
    return [
        {
            "type": "function",
            "function": {
                "name": t["name"],
                "description": t["description"],
                "parameters": t["parameters"],
            },
        }
        for t in tools
    ]


def _openai_messages(system: str, messages: list[dict]) -> list[dict]:
    out: list[dict] = [{"role": "system", "content": system}]
    for m in messages:
        if m["role"] == "user":
            out.append({"role": "user", "content": m["content"]})
        elif m["role"] == "assistant":
            step = m["step"]
            msg: dict = {"role": "assistant", "content": step.text or None}
            if step.tool_calls:
                msg["tool_calls"] = [
                    {
                        "id": tc.id,
                        "type": "function",
                        "function": {"name": tc.name, "arguments": json.dumps(tc.args)},
                    }
                    for tc in step.tool_calls
                ]
            out.append(msg)
        elif m["role"] == "tool":
            for r in m["results"]:
                out.append({"role": "tool", "tool_call_id": r["id"], "content": r["content"]})
    return out


def _step_from_openai(resp) -> Step:
    choice = resp.choices[0]
    msg = choice.message
    calls = [
        ToolCall(id=tc.id, name=tc.function.name, args=_json_args(tc.function.arguments))
        for tc in (msg.tool_calls or [])
    ]
    u = getattr(resp, "usage", None)
    return Step(
        text=msg.content or "",
        tool_calls=calls,
        input_tokens=getattr(u, "prompt_tokens", 0) if u else 0,
        output_tokens=getattr(u, "completion_tokens", 0) if u else 0,
        stop_reason=getattr(choice, "finish_reason", "") or "",
    )


class OpenAIModel:
    def __init__(self, name: str, model_id: str, temperature: float, max_tokens: int):
        try:
            from openai import OpenAI
        except ImportError as e:  # pragma: no cover
            raise SystemExit("pip install openai  (uv sync --extra providers)") from e
        if not os.environ.get("OPENAI_API_KEY"):
            raise SystemExit("OPENAI_API_KEY is not set")
        self.name = name
        self.model_id = model_id
        self.temperature = temperature
        self.max_tokens = max_tokens
        self._client = OpenAI(timeout=120.0, max_retries=4)

    def step(self, system: str, messages: list[dict], tools: list[dict]) -> Step:
        resp = self._client.chat.completions.create(
            model=self.model_id,
            messages=_openai_messages(system, messages),
            tools=_openai_tools(tools),
            tool_choice="auto",
            max_tokens=self.max_tokens,
            temperature=self.temperature,
        )
        return _step_from_openai(resp)


# ---- Gemini tool-use translation ------------------------------------------------------------
# google-genai uses "model"/"user" roles, a system_instruction, and function_call/function_response
# parts (matched by function *name*, not an id). The converters emit plain dicts (the SDK coerces
# them), so they stay import-free and testable. NOTE for smoke time: verify the SDK accepts dict-form
# tools/contents for the pinned version; if not, wrap with google.genai.types in GeminiModel.step.


def _gemini_tools(tools: list[dict]) -> list[dict]:
    return [
        {
            "function_declarations": [
                {"name": t["name"], "description": t["description"], "parameters": t["parameters"]}
                for t in tools
            ]
        }
    ]


def _gemini_contents(messages: list[dict]) -> list[dict]:
    contents: list[dict] = []
    id_to_name: dict[str, str] = {}  # Gemini keys function_response by name, not id
    for m in messages:
        if m["role"] == "user":
            contents.append({"role": "user", "parts": [{"text": m["content"]}]})
        elif m["role"] == "assistant":
            step = m["step"]
            parts: list[dict] = []
            if step.text:
                parts.append({"text": step.text})
            for tc in step.tool_calls:
                parts.append({"function_call": {"name": tc.name, "args": tc.args}})
                id_to_name[tc.id] = tc.name
            contents.append({"role": "model", "parts": parts})
        elif m["role"] == "tool":
            parts = [
                {
                    "function_response": {
                        "name": id_to_name.get(r["id"], r.get("name", "")),
                        "response": {"result": r["content"]},
                    }
                }
                for r in m["results"]
            ]
            contents.append({"role": "user", "parts": parts})
    return contents


def _step_from_gemini(resp) -> Step:
    cand = resp.candidates[0]
    text = ""
    calls: list[ToolCall] = []
    for i, part in enumerate(cand.content.parts):
        fc = getattr(part, "function_call", None)
        if fc is not None:
            args = dict(fc.args) if getattr(fc, "args", None) else {}
            calls.append(ToolCall(id=f"{fc.name}-{i}", name=fc.name, args=args))
        elif getattr(part, "text", None):
            text += part.text
    u = getattr(resp, "usage_metadata", None)
    return Step(
        text=text,
        tool_calls=calls,
        input_tokens=getattr(u, "prompt_token_count", 0) if u else 0,
        output_tokens=getattr(u, "candidates_token_count", 0) if u else 0,
        stop_reason=str(getattr(cand, "finish_reason", "") or ""),
    )


class GeminiModel:
    def __init__(self, name: str, model_id: str, temperature: float, max_tokens: int):
        try:
            from google import genai
        except ImportError as e:  # pragma: no cover
            raise SystemExit("pip install google-genai  (uv sync --extra providers)") from e
        if not (os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY")):
            raise SystemExit("GEMINI_API_KEY (or GOOGLE_API_KEY) is not set")
        self.name = name
        self.model_id = model_id
        self.temperature = temperature
        self.max_tokens = max_tokens
        self._client = genai.Client()

    def step(self, system: str, messages: list[dict], tools: list[dict]) -> Step:
        from google.genai import types

        config = types.GenerateContentConfig(
            system_instruction=system,
            tools=_gemini_tools(tools),
            temperature=self.temperature,
            max_output_tokens=self.max_tokens,
        )
        resp = self._client.models.generate_content(
            model=self.model_id, contents=_gemini_contents(messages), config=config
        )
        return _step_from_gemini(resp)


def build_model(
    name: str, spec: dict, *, temperature: float, max_tokens: int, mode: str = "single"
) -> Model:
    provider = spec.get("provider")
    if provider == "mock":
        if mode == "multi":
            return MockToolModel(
                name=name, default=spec.get("default", ""), replies=spec.get("replies")
            )
        return MockModel(name=name, default=spec.get("default", ""), replies=spec.get("replies"))
    cls = {"anthropic": AnthropicModel, "openai": OpenAIModel, "gemini": GeminiModel}.get(provider)
    if cls is not None:
        return cls(
            name=name,
            model_id=spec["model_id"],
            temperature=spec.get("temperature", temperature),
            max_tokens=spec.get("max_tokens", max_tokens),
        )
    raise ValueError(f"unknown provider {provider!r} for model {name!r}")
