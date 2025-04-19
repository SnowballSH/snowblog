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
 * Decodes common HTML entities to their original characters
 */
function decodeHtmlEntities(text: string): string {
  return text
    .replace(/&amp;/g, "&")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/\\&amp;/g, "\\&");
  // Note: We don't process backslashes here as they're now handled via placeholders
}

/**
 * Processes LaTeX expressions in HTML content
 */
export async function renderLatex(html: string): Promise<string> {
  if (!html) return html;

  try {
    // Render LaTeX blocks wrapped in $$...$$
    html = html.replace(/\$\$(.*?)\$\$/gs, (_, expr) => {
      try {
        // Decode HTML entities before passing to KaTeX
        const decodedExpr = decodeHtmlEntities(expr.trim());
        return katex.renderToString(decodedExpr, {
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
        // Decode HTML entities before passing to KaTeX
        const decodedExpr = decodeHtmlEntities(expr.trim());
        return katex.renderToString(decodedExpr, {
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

// Unique placeholder for double backslashes in LaTeX
const DOUBLE_BACKSLASH_PLACEHOLDER = "DOUBLEBACKSLASHPLACEHOLDER";

/**
 * Converts markdown to HTML with syntax highlighting and LaTeX support
 */
export async function renderMarkdown(
  markdown: string | undefined,
): Promise<string> {
  if (!markdown) {
    return "<p>No content available</p>";
  }

  // Pre-process markdown to protect double backslashes in LaTeX blocks
  let processedMarkdown = markdown;

  // Replace \\ with a placeholder in block LaTeX
  processedMarkdown = processedMarkdown.replace(
    /(\$\$[\s\S]*?\$\$)/g,
    (match) => {
      return match.replace(/\\\\/g, DOUBLE_BACKSLASH_PLACEHOLDER);
    },
  );

  // Replace \\ with a placeholder in inline LaTeX
  processedMarkdown = processedMarkdown.replace(/(\$[^\$\n]+?\$)/g, (match) => {
    return match.replace(/\\\\/g, DOUBLE_BACKSLASH_PLACEHOLDER);
  });

  // Convert processed markdown to HTML with syntax highlighting
  let html = await marked.parse(processedMarkdown);

  // Replace placeholders back to double backslashes
  html = html.replace(new RegExp(DOUBLE_BACKSLASH_PLACEHOLDER, "g"), "\\\\");

  // Then process any LaTeX in the HTML
  html = await renderLatex(html);

  return html;
}
