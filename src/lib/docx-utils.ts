import {
  Document,
  Packer,
  Paragraph,
  TextRun,
  HeadingLevel,
  AlignmentType,
} from "docx";
import { saveAs } from "file-saver";
import mammoth from "mammoth";

/**
 * Export HTML content to a DOCX file
 */
export async function exportToDocx(
  title: string,
  htmlContent: string
): Promise<void> {
  // Parse HTML content and convert to docx elements
  const children = parseHtmlToDocx(htmlContent);

  const doc = new Document({
    sections: [
      {
        properties: {},
        children: [
          // Title
          new Paragraph({
            text: title,
            heading: HeadingLevel.TITLE,
            spacing: { after: 400 },
          }),
          ...children,
        ],
      },
    ],
  });

  const blob = await Packer.toBlob(doc);
  const filename = sanitizeFilename(title) + ".docx";
  saveAs(blob, filename);
}

/**
 * Import a DOCX file and convert to HTML
 */
export async function importFromDocx(
  file: File
): Promise<{ title: string; content: string }> {
  const arrayBuffer = await file.arrayBuffer();
  const result = await mammoth.convertToHtml({ arrayBuffer });

  // Extract title from filename (without extension)
  const title = file.name.replace(/\.docx$/i, "");

  return {
    title,
    content: result.value,
  };
}

/**
 * Parse HTML string and convert to docx Paragraph elements
 */
function parseHtmlToDocx(html: string): Paragraph[] {
  const paragraphs: Paragraph[] = [];

  // Create a temporary DOM element to parse HTML
  const parser = new DOMParser();
  const doc = parser.parseFromString(html, "text/html");
  const body = doc.body;

  const processNode = (node: Node): void => {
    if (node.nodeType === Node.TEXT_NODE) {
      const text = node.textContent?.trim();
      if (text) {
        paragraphs.push(new Paragraph({ children: [new TextRun(text)] }));
      }
      return;
    }

    if (node.nodeType !== Node.ELEMENT_NODE) return;

    const element = node as Element;
    const tagName = element.tagName.toLowerCase();

    switch (tagName) {
      case "h1":
        paragraphs.push(
          new Paragraph({
            text: element.textContent || "",
            heading: HeadingLevel.HEADING_1,
            spacing: { before: 400, after: 200 },
          })
        );
        break;

      case "h2":
        paragraphs.push(
          new Paragraph({
            text: element.textContent || "",
            heading: HeadingLevel.HEADING_2,
            spacing: { before: 300, after: 150 },
          })
        );
        break;

      case "h3":
        paragraphs.push(
          new Paragraph({
            text: element.textContent || "",
            heading: HeadingLevel.HEADING_3,
            spacing: { before: 200, after: 100 },
          })
        );
        break;

      case "p":
        paragraphs.push(
          new Paragraph({
            children: parseInlineElements(element),
            spacing: { after: 200 },
          })
        );
        break;

      case "ul":
      case "ol":
        Array.from(element.children).forEach((li, index) => {
          const bullet = tagName === "ul" ? "• " : `${index + 1}. `;
          paragraphs.push(
            new Paragraph({
              children: [
                new TextRun(bullet),
                ...parseInlineElements(li as Element),
              ],
              indent: { left: 720 },
              spacing: { after: 100 },
            })
          );
        });
        break;

      case "blockquote":
        paragraphs.push(
          new Paragraph({
            children: parseInlineElements(element),
            indent: { left: 720 },
            spacing: { before: 200, after: 200 },
          })
        );
        break;

      case "pre":
        const codeText = element.textContent || "";
        codeText.split("\n").forEach((line) => {
          paragraphs.push(
            new Paragraph({
              children: [
                new TextRun({
                  text: line,
                  font: "Courier New",
                  size: 20,
                }),
              ],
              spacing: { after: 0 },
            })
          );
        });
        break;

      case "br":
        paragraphs.push(new Paragraph({ children: [] }));
        break;

      default:
        // Process children for unknown elements
        Array.from(element.childNodes).forEach(processNode);
    }
  };

  Array.from(body.childNodes).forEach(processNode);

  return paragraphs;
}

/**
 * Parse inline elements (bold, italic, underline, etc.) into TextRun objects
 */
function parseInlineElements(element: Element): TextRun[] {
  const runs: TextRun[] = [];

  const processInline = (
    node: Node,
    styles: { bold?: boolean; italic?: boolean; underline?: boolean; strike?: boolean }
  ): void => {
    if (node.nodeType === Node.TEXT_NODE) {
      const text = node.textContent || "";
      if (text) {
        runs.push(
          new TextRun({
            text,
            bold: styles.bold,
            italics: styles.italic,
            underline: styles.underline ? {} : undefined,
            strike: styles.strike,
          })
        );
      }
      return;
    }

    if (node.nodeType !== Node.ELEMENT_NODE) return;

    const el = node as Element;
    const tag = el.tagName.toLowerCase();

    const newStyles = { ...styles };
    if (tag === "strong" || tag === "b") newStyles.bold = true;
    if (tag === "em" || tag === "i") newStyles.italic = true;
    if (tag === "u") newStyles.underline = true;
    if (tag === "s" || tag === "strike" || tag === "del") newStyles.strike = true;

    if (tag === "code") {
      runs.push(
        new TextRun({
          text: el.textContent || "",
          font: "Courier New",
          size: 20,
        })
      );
      return;
    }

    if (tag === "br") {
      runs.push(new TextRun({ text: "", break: 1 }));
      return;
    }

    Array.from(el.childNodes).forEach((child) => processInline(child, newStyles));
  };

  Array.from(element.childNodes).forEach((child) =>
    processInline(child, {})
  );

  return runs;
}

/**
 * Sanitize a string for use as a filename
 */
function sanitizeFilename(name: string): string {
  return name
    .replace(/[<>:"/\\|?*]/g, "")
    .replace(/\s+/g, "_")
    .substring(0, 100) || "document";
}
