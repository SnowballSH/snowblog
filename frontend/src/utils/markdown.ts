// src/utils/markdown.ts
import { Marked } from "marked";
import { markedHighlight } from "marked-highlight";
import hljs from "highlight.js";
import katex from "katex";

// Configure marked with syntax highlighting
const marked = new Marked(
  markedHighlight({
    langPrefix: "hljs language-",
    highlight(code, lang) {
      const language = hljs.getLanguage(lang) ? lang : "plaintext";
      return hljs.highlight(code, { language }).value;
    },
  }),
);

/**
 * Processes LaTeX expressions in HTML content
 */
export async function renderLatex(html: string): Promise<string> {
  if (!html) return html;

  try {
    // Render LaTeX blocks wrapped in $$...$$
    html = html.replace(/\$\$(.*?)\$\$/gs, (_, expr) => {
      try {
        return katex.renderToString(expr.trim(), {
          displayMode: true,
          strict: false,
          trust: true,
        });
      } catch (error) {
        console.error("KaTeX block error:", error);
        return `<pre>${expr}</pre>`;
      }
    });

    // Render inline LaTeX wrapped in $...$
    html = html.replace(/\$(.*?)\$/gs, (_, expr) => {
      try {
        return katex.renderToString(expr.trim(), {
          displayMode: false,
          strict: false,
          trust: true,
        });
      } catch (error) {
        console.error("KaTeX inline error:", error);
        return `<pre>${expr}</pre>`;
      }
    });

    return html;
  } catch (error) {
    console.error("LaTeX rendering error:", error);
    return html;
  }
}

/**
 * Converts markdown to HTML with syntax highlighting and LaTeX support
 */
export async function renderMarkdown(
  markdown: string | undefined,
): Promise<string> {
  if (!markdown) {
    return "<p>No content available</p>";
  }

  // First convert markdown to HTML with syntax highlighting
  let html = await marked.parse(markdown);

  // Then process any LaTeX in the HTML
  html = await renderLatex(html);

  return html;
}
