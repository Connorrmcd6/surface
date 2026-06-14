// Reference STALE solution: trusts the doc's 30000 ms default (the misled answer).
export function requestBudgetMs(): number {
  return 30000 * 3;
}
