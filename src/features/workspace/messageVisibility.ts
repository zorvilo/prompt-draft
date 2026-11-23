export function visibleMessageIds(
  viewport: { top: number; bottom: number },
  messages: { id: string; top: number; bottom: number }[],
): string[] {
  return messages
    .filter(
      (message) =>
        message.bottom > viewport.top && message.top < viewport.bottom,
    )
    .map((message) => message.id);
}
