import type { Intervention, InterventionQuestion, Snapshot } from './bridge';

export function pendingIntervention(snapshot: Snapshot | undefined, nonce: string): Intervention | undefined {
  if (!snapshot || snapshot.scenario !== 'real') return undefined;
  const request = snapshot.interventions.find(candidate => candidate.nonce === nonce && candidate.status === 'pending');
  if (!request || request.generation === '' || request.nonce === '') return undefined;
  const claude = request.generation.startsWith('claude:');
  const identityMatches = claude
    ? request.sessionId === `claude:${request.threadId}`
    : request.sessionId === `codex:${request.threadId}`;
  if (!identityMatches) return undefined;
  const kind = claude ? 'claude' : 'codex';
  if (!snapshot.sessions.some(session => session.id === request.sessionId && session.kind === kind && !session.hidden)) return undefined;
  return request;
}

export function interventionContext(request: Intervention) {
  return {
    nonce: request.nonce,
    generation: request.generation,
    sessionId: request.sessionId,
    threadId: request.threadId,
    turnId: request.turnId,
    itemId: request.itemId,
  };
}

export type AnswerFields = Map<string, { value: string; checked: boolean; focused: boolean }>;
type AnswerInput = HTMLInputElement;
function answerKey(input: AnswerInput): string {
  return input.dataset.freeAnswer
    ? `free:${input.dataset.freeAnswer}`
    : input.dataset.answerOption !== undefined
      ? `choice:${input.dataset.questionAnswer}:${input.name}:option:${input.dataset.answerOption}`
      : `choice:${input.dataset.questionAnswer}:${input.name}`;
}

export function questionAnswer(question: InterventionQuestion, freeText: string, selected: string[]): string | string[] | undefined {
  const text = freeText.trim();
  if (text) return text;
  if (question.multiSelect) return selected.length ? selected : undefined;
  return selected[0];
}
export function captureInterventionAnswers(inputs: Iterable<AnswerInput>, activeElement: Element | null): AnswerFields {
  const result: AnswerFields = new Map();
  for (const input of inputs) {
    const nonce = input.closest<HTMLElement>('[data-request]')?.dataset.request;
    if (nonce) result.set(`${nonce}\0${answerKey(input)}`, { value: input.value, checked: input.checked, focused: activeElement === input });
  }
  return result;
}
export function restoreInterventionAnswers(inputs: Iterable<AnswerInput>, saved: AnswerFields): void {
  for (const input of inputs) {
    const nonce = input.closest<HTMLElement>('[data-request]')?.dataset.request;
    const value = nonce ? saved.get(`${nonce}\0${answerKey(input)}`) : undefined;
    if (!value) continue;
    input.value = value.value;
    input.checked = value.checked;
    if (value.focused) input.focus({ preventScroll: true });
  }
}
