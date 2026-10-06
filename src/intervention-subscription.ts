type SubscriptionButton = {
  dataset: { action?: string; enabled?: string };
  disabled: boolean;
  setAttribute(name: string, value: string): void;
};

export function runInterventionSubscriptionClick<T>(
  button: SubscriptionButton,
  sessionId: string,
  subscribe: (sessionId: string, enabled: boolean) => Promise<T>,
): Promise<T> | undefined {
  if (button.dataset.action !== 'connect-interventions' || button.disabled) return undefined;
  button.disabled = true;
  button.setAttribute('aria-busy', 'true');
  return subscribe(sessionId, button.dataset.enabled === 'true');
}
