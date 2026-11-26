import type { ModificationMode } from "@/api/desktop";
export interface GenerationRequest {
  text: string;
  userMessage?: string;
  mode?: ModificationMode;
  model?: string;
  providerName?: string;
  templateId?: string;
  templateName?: string;
}
