/** The semantic action emitted by the standard UI-session diagnostic. */
export const STANDARD_SESSION_ACTION = "sample.session.action";

/** The exact version 1 document used by the standard UI-session diagnostic. */
export const STANDARD_SESSION_DOCUMENT =
  '{"format":"anodrel.ui.document.v1","root":{"id":"sample.session.root","kind":"stack","axis":"vertical","padding":{"left":56,"top":56,"right":56,"bottom":56},"gap":16,"surfaceTone":"plain","children":[{"id":"sample.session.eyebrow","kind":"text","value":"AUTHENTICATED ANODREL SESSION","fontSize":14,"tone":"accent"},{"id":"sample.session.title","kind":"text","value":"Native document delivered","fontSize":28,"tone":"primary"},{"id":"sample.session.detail","kind":"text","value":"This view came through the private pipe and remains free of native action authority.","fontSize":16,"tone":"secondary"},{"id":"sample.session.action","kind":"action","label":"Visual-only semantic action","fontSize":16,"enabled":true,"tone":"accent"}]}}';

/** The semantic action emitted after the version 2 scroll diagnostic reaches its end. */
export const SCROLL_SESSION_ACTION = "sample.scroll.complete";

/**
 * The exact version 2 document used to verify host-retained native scrolling.
 *
 * The application supplies only content. The viewport offset, wheel input, and
 * page navigation remain local to the host-rendered session view.
 */
export const SCROLL_SESSION_DOCUMENT = JSON.stringify({
  format: "anodrel.ui.document.v2",
  root: {
    id: "sample.scroll.viewport",
    kind: "scroll",
    child: {
      id: "sample.scroll.content",
      kind: "stack",
      axis: "vertical",
      padding: { left: 56, top: 56, right: 56, bottom: 56 },
      gap: 16,
      surfaceTone: "plain",
      children: [
        {
          id: "sample.scroll.eyebrow",
          kind: "text",
          value: "AUTHENTICATED ANODREL SCROLL SESSION",
          fontSize: 14,
          tone: "accent",
        },
        {
          id: "sample.scroll.title",
          kind: "text",
          value: "Host-retained native scrolling",
          fontSize: 28,
          tone: "primary",
        },
        {
          id: "sample.scroll.detail",
          kind: "text",
          value:
            "Use the mouse wheel or Page Down. The viewport position stays in the host and never crosses the protocol boundary.",
          fontSize: 16,
          tone: "secondary",
        },
        ...Array.from({ length: 14 }, (_, index) => ({
          id: `sample.scroll.note.${index + 1}`,
          kind: "text" as const,
          value: `Scroll checkpoint ${index + 1}: content remains clipped to this native viewport.`,
          fontSize: 16,
          tone: "secondary" as const,
        })),
        {
          id: SCROLL_SESSION_ACTION,
          kind: "action",
          label: "Complete scroll diagnostic",
          fontSize: 16,
          enabled: true,
          tone: "accent",
        },
      ],
    },
  },
});
