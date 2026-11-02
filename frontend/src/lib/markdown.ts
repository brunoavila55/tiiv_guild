// Subconjunto de markdown para textos escritos pela guilda (descrição da página pública).
// Gera uma árvore que vira elementos React: HTML cru nunca é interpretado, e links só
// saem para http(s). Sem dependência: o que a guilda escreve não precisa de mais que isso.

export type Inline =
  | { type: "text"; text: string }
  | { type: "break" }
  | { type: "code"; text: string }
  | { type: "strong"; children: Inline[] }
  | { type: "em"; children: Inline[] }
  | { type: "link"; href: string; children: Inline[] };

export type Block =
  | { type: "heading"; level: 1 | 2 | 3; children: Inline[] }
  | { type: "paragraph"; children: Inline[] }
  | { type: "quote"; children: Inline[] }
  | { type: "list"; ordered: boolean; items: Inline[][] }
  | { type: "code"; text: string }
  | { type: "hr" };

const HEADING = /^(#{1,6})\s+(.*?)\s*#*\s*$/;
const HR = /^\s*([-*_])(\s*\1){2,}\s*$/;
const QUOTE = /^\s*>\s?(.*)$/;
const BULLET = /^\s*[-*+]\s+(.*)$/;
const NUMBERED = /^\s*\d{1,9}[.)]\s+(.*)$/;
const FENCE = /^\s*```/;

const startsBlock = (line: string) =>
  HEADING.test(line) || HR.test(line) || QUOTE.test(line) || BULLET.test(line) || NUMBERED.test(line) || FENCE.test(line);

export function parseMarkdown(src: string): Block[] {
  const lines = src.replace(/\r\n?/g, "\n").split("\n");
  const blocks: Block[] = [];
  let i = 0;
  const at = (n: number) => lines[n] ?? "";
  // Junta as linhas consecutivas que casam com `re`, devolvendo o conteúdo de cada uma.
  const take = (re: RegExp) => {
    const out: string[] = [];
    let m: RegExpExecArray | null;
    while (i < lines.length && (m = re.exec(at(i)))) {
      out.push(m[1] ?? "");
      i++;
    }
    return out;
  };

  while (i < lines.length) {
    const line = at(i);
    if (!line.trim()) {
      i++;
    } else if (FENCE.test(line)) {
      i++;
      const code: string[] = [];
      while (i < lines.length && !FENCE.test(at(i))) code.push(at(i++));
      i++;
      blocks.push({ type: "code", text: code.join("\n") });
    } else if (HR.test(line)) {
      i++;
      blocks.push({ type: "hr" });
    } else if (HEADING.test(line)) {
      const [, hashes = "", text = ""] = HEADING.exec(line) ?? [];
      i++;
      const level = Math.min(hashes.length, 3) as 1 | 2 | 3;
      blocks.push({ type: "heading", level, children: parseInline(text) });
    } else if (QUOTE.test(line)) {
      blocks.push({ type: "quote", children: inlineLines(take(QUOTE)) });
    } else if (BULLET.test(line)) {
      blocks.push({ type: "list", ordered: false, items: take(BULLET).map(parseInline) });
    } else if (NUMBERED.test(line)) {
      blocks.push({ type: "list", ordered: true, items: take(NUMBERED).map(parseInline) });
    } else {
      const para: string[] = [];
      while (i < lines.length && at(i).trim() && !startsBlock(at(i))) para.push(at(i++).trim());
      blocks.push({ type: "paragraph", children: inlineLines(para) });
    }
  }
  return blocks;
}

/** Linhas de um mesmo bloco: cada quebra de linha do texto vira quebra na tela. */
function inlineLines(lines: string[]): Inline[] {
  return lines.flatMap((l, idx) => (idx === 0 ? parseInline(l) : [{ type: "break" } as const, ...parseInline(l)]));
}

const INLINE = /`([^`]+)`|\[([^\]]+)\]\(([^)\s]+)\)|\*\*(.+?)\*\*|__(.+?)__|\*(.+?)\*|(?<!\w)_(.+?)_(?!\w)/g;

export function parseInline(src: string): Inline[] {
  const out: Inline[] = [];
  let last = 0;
  for (const m of src.matchAll(INLINE)) {
    if (m.index > last) out.push({ type: "text", text: src.slice(last, m.index) });
    last = m.index + m[0].length;
    const [, code, label, href, strong1, strong2, em1, em2] = m;
    if (code !== undefined) out.push({ type: "code", text: code });
    else if (label !== undefined) {
      const children = parseInline(label);
      if (href !== undefined && safeHref(href)) out.push({ type: "link", href, children });
      else out.push(...children);
    } else if (strong1 !== undefined || strong2 !== undefined) out.push({ type: "strong", children: parseInline(strong1 ?? strong2 ?? "") });
    else out.push({ type: "em", children: parseInline(em1 ?? em2 ?? "") });
  }
  if (last < src.length) out.push({ type: "text", text: src.slice(last) });
  return out;
}

/** Só links absolutos http(s): nada de `javascript:`, `data:` ou caminhos do nosso domínio. */
export function safeHref(href: string): boolean {
  try {
    const url = new URL(href);
    return url.protocol === "https:" || url.protocol === "http:";
  } catch {
    return false;
  }
}
