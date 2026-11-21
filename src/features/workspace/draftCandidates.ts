import type { Draft, Version } from "../../lib/tauri";

export function draftCandidates(draft: Draft): Version[] {
  return draft.versions.filter(
    (version) =>
      !version.instruction &&
      version.source === draft.source &&
      version.model !== "手动编辑" &&
      version.model !== "恢复前的草稿",
  );
}

export function selectCandidate(draft: Draft, candidate: Version) {
  if (
    draft.result.trim() &&
    !draft.versions.some(
      (version) =>
        version.result === draft.result && version.source === draft.source,
    )
  ) {
    draft.versions.unshift({
      id: crypto.randomUUID(),
      source: draft.source,
      result: draft.result,
      question: draft.question,
      model: "手动编辑",
      created: Date.now(),
    });
  }
  draft.result = candidate.result;
  draft.question = candidate.question;
  draft.updated = Date.now();
}
