export function routeTemplateWheel(event: WheelEvent) {
  if (event.ctrlKey || event.defaultPrevented || !event.cancelable) return;
  const field = event.currentTarget as HTMLElement;
  const page = field.closest<HTMLElement>(".utility-scroll, .main-surface");
  if (!page || field.contains(field.ownerDocument.activeElement)) return;

  event.preventDefault();
  event.stopPropagation();
  const unit =
    event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? page.clientHeight : 1;
  page.scrollBy({ top: event.deltaY * unit, left: event.deltaX * unit });
}
