import type { Intervention, Snapshot } from './bridge';

export function pendingIntervention(snapshot: Snapshot | undefined, nonce: string): Intervention | undefined {
  if (!snapshot || snapshot.scenario !== 'real') return undefined;
  const request = snapshot.interventions.find(candidate => candidate.nonce === nonce && candidate.status === 'pending');
  if (!request || request.generation === '' || request.nonce === '' || request.sessionId !== `codex:${request.threadId}`) return undefined;
  if (!snapshot.sessions.some(session => session.id === request.sessionId && session.kind === 'codex' && !session.hidden)) return undefined;
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
