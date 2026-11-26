import MarkdownIt from "markdown-it";
const markdown = new MarkdownIt({ html: false, breaks: true });
markdown.renderer.rules.image = (tokens, index) =>
  markdown.utils.escapeHtml(tokens[index].content);
markdown.renderer.rules.link_open = () => "<span>";
markdown.renderer.rules.link_close = () => "</span>";
export const renderMarkdown = (text: string) => markdown.render(text);
