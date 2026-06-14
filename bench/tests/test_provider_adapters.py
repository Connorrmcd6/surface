"""Offline tests for the OpenAI + Gemini tool-use translation (no network — pure converters and a
fake response). Live round-trips are smoke-tested manually, not in the suite (#107)."""

from __future__ import annotations

import json
from types import SimpleNamespace

from surface_bench.models import (
    Step,
    ToolCall,
    _gemini_contents,
    _gemini_tools,
    _openai_messages,
    _openai_tools,
    _step_from_gemini,
    _step_from_openai,
    build_model,
)
from surface_bench.tools_runtime import TOOL_SPECS


# ---- OpenAI ---------------------------------------------------------------------------------


def test_openai_tools_and_messages() -> None:
    tools = _openai_tools(TOOL_SPECS)
    assert all(t["type"] == "function" for t in tools)
    assert {t["function"]["name"] for t in tools} == {"list_dir", "read_file", "grep", "final_answer"}
    assert tools[0]["function"]["parameters"] == TOOL_SPECS[0]["parameters"]

    asst = Step(tool_calls=[ToolCall(id="c1", name="read_file", args={"path": "code/x.py"})])
    messages = [
        {"role": "user", "content": "task"},
        {"role": "assistant", "step": asst},
        {"role": "tool", "results": [{"id": "c1", "content": "WINDOW_LIMIT = 10"}]},
    ]
    out = _openai_messages("sys", messages)
    assert out[0] == {"role": "system", "content": "sys"}
    assert out[1] == {"role": "user", "content": "task"}
    tc = out[2]["tool_calls"][0]
    assert tc["id"] == "c1" and tc["function"]["name"] == "read_file"
    assert json.loads(tc["function"]["arguments"]) == {"path": "code/x.py"}
    assert out[3] == {"role": "tool", "tool_call_id": "c1", "content": "WINDOW_LIMIT = 10"}


def test_step_from_openai_parses_tool_calls() -> None:
    resp = SimpleNamespace(
        choices=[
            SimpleNamespace(
                message=SimpleNamespace(
                    content="checking",
                    tool_calls=[
                        SimpleNamespace(
                            id="c9",
                            function=SimpleNamespace(
                                name="read_file", arguments='{"path": "code/x.py"}'
                            ),
                        )
                    ],
                ),
                finish_reason="tool_calls",
            )
        ],
        usage=SimpleNamespace(prompt_tokens=100, completion_tokens=12),
    )
    step = _step_from_openai(resp)
    assert step.text == "checking"
    assert step.tool_calls == [ToolCall(id="c9", name="read_file", args={"path": "code/x.py"})]
    assert (step.input_tokens, step.output_tokens) == (100, 12)
    assert step.stop_reason == "tool_calls"


def test_step_from_openai_tolerates_no_tool_calls_and_bad_json() -> None:
    resp = SimpleNamespace(
        choices=[SimpleNamespace(message=SimpleNamespace(content="final", tool_calls=None), finish_reason="stop")],
        usage=SimpleNamespace(prompt_tokens=5, completion_tokens=2),
    )
    assert _step_from_openai(resp).tool_calls == []


# ---- Gemini ---------------------------------------------------------------------------------


def test_gemini_tools_and_function_response_name_resolution() -> None:
    tools = _gemini_tools(TOOL_SPECS)
    decls = tools[0]["function_declarations"]
    assert {d["name"] for d in decls} == {"list_dir", "read_file", "grep", "final_answer"}

    asst = Step(tool_calls=[ToolCall(id="g1", name="read_file", args={"path": "code/x.py"})])
    messages = [
        {"role": "user", "content": "task"},
        {"role": "assistant", "step": asst},
        {"role": "tool", "results": [{"id": "g1", "content": "WINDOW_LIMIT = 10"}]},
    ]
    contents = _gemini_contents(messages)
    assert [c["role"] for c in contents] == ["user", "model", "user"]
    assert contents[1]["parts"][0]["function_call"] == {"name": "read_file", "args": {"path": "code/x.py"}}
    # the tool result is keyed by function NAME (resolved from the prior call's id), not the id
    fr = contents[2]["parts"][0]["function_response"]
    assert fr["name"] == "read_file"
    assert fr["response"] == {"result": "WINDOW_LIMIT = 10"}


def test_step_from_gemini_parses_parts() -> None:
    resp = SimpleNamespace(
        candidates=[
            SimpleNamespace(
                content=SimpleNamespace(
                    parts=[
                        SimpleNamespace(function_call=None, text="let me look"),
                        SimpleNamespace(
                            function_call=SimpleNamespace(name="read_file", args={"path": "code/x.py"}),
                            text=None,
                        ),
                    ]
                ),
                finish_reason="STOP",
            )
        ],
        usage_metadata=SimpleNamespace(prompt_token_count=80, candidates_token_count=9),
    )
    step = _step_from_gemini(resp)
    assert step.text == "let me look"
    assert step.tool_calls[0].name == "read_file" and step.tool_calls[0].args == {"path": "code/x.py"}
    assert (step.input_tokens, step.output_tokens) == (80, 9)
    assert step.stop_reason == "STOP"


# ---- registry -------------------------------------------------------------------------------


def test_build_model_unknown_provider_still_raises() -> None:
    try:
        build_model("x", {"provider": "nope"}, temperature=1.0, max_tokens=10)
    except ValueError as e:
        assert "unknown provider" in str(e)
    else:
        raise AssertionError("expected ValueError")
