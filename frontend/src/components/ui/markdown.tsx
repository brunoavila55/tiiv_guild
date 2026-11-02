import { Fragment, useMemo, type ReactNode } from "react";

import { parseMarkdown, type Block, type Inline } from "@/lib/markdown";
import { cn } from "@/lib/utils";

/** Renderiza o subconjunto seguro de markdown de `@/lib/markdown`. */
export function Markdown({ source, className }: { source: string; className?: string }) {
  const blocks = useMemo(() => parseMarkdown(source), [source]);
  return <div className={cn("flex flex-col gap-4 leading-relaxed", className)}>{blocks.map(renderBlock)}</div>;
}

const headings = {
  1: "text-xl font-semibold text-fg",
  2: "text-lg font-semibold text-fg",
  3: "font-semibold text-fg",
} as const;

function renderBlock(b: Block, key: number): ReactNode {
  switch (b.type) {
    case "heading": {
      // O h1 da página é o nome da guilda: os títulos do texto começam no h2.
      const Tag = ({ 1: "h2", 2: "h3", 3: "h4" } as const)[b.level];
      return <Tag key={key} className={headings[b.level]}>{renderInline(b.children)}</Tag>;
    }
    case "paragraph":
      return <p key={key}>{renderInline(b.children)}</p>;
    case "quote":
      return <blockquote key={key} className="border-l-4 border-primary/60 pl-4 italic">{renderInline(b.children)}</blockquote>;
    case "list": {
      const Tag = b.ordered ? "ol" : "ul";
      return (
        <Tag key={key} className={cn("flex flex-col gap-1 pl-6", b.ordered ? "list-decimal" : "list-disc")}>
          {b.items.map((item, i) => <li key={i}>{renderInline(item)}</li>)}
        </Tag>
      );
    }
    case "code":
      return <pre key={key} className="overflow-x-auto rounded-lg bg-surface-2 p-4 text-sm"><code>{b.text}</code></pre>;
    case "hr":
      return <hr key={key} className="border-border" />;
  }
}

function renderInline(nodes: Inline[]): ReactNode {
  return nodes.map((n, key) => {
    switch (n.type) {
      case "text":
        return <Fragment key={key}>{n.text}</Fragment>;
      case "break":
        return <br key={key} />;
      case "code":
        return <code key={key} className="rounded bg-surface-2 px-1.5 py-0.5 text-[0.9em]">{n.text}</code>;
      case "strong":
        return <strong key={key} className="font-semibold text-fg">{renderInline(n.children)}</strong>;
      case "em":
        return <em key={key}>{renderInline(n.children)}</em>;
      case "link":
        return (
          <a key={key} href={n.href} target="_blank" rel="noopener noreferrer nofollow ugc" className="text-primary underline underline-offset-2 hover:brightness-125">
            {renderInline(n.children)}
          </a>
        );
    }
  });
}
