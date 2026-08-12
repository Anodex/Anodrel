/** The semantic action emitted by the standard UI-session diagnostic. */
export const STANDARD_SESSION_ACTION = "sample.session.action";

/** The exact version 1 document used by the standard UI-session diagnostic. */
export const STANDARD_SESSION_DOCUMENT =
  '{"format":"anodrel.ui.document.v1","root":{"id":"sample.session.root","kind":"stack","axis":"vertical","padding":{"left":56,"top":56,"right":56,"bottom":56},"gap":16,"surfaceTone":"plain","children":[{"id":"sample.session.eyebrow","kind":"text","value":"AUTHENTICATED ANODREL SESSION","fontSize":14,"tone":"accent"},{"id":"sample.session.title","kind":"text","value":"Native document delivered","fontSize":28,"tone":"primary"},{"id":"sample.session.detail","kind":"text","value":"This view came through the private pipe and remains free of native action authority.","fontSize":16,"tone":"secondary"},{"id":"sample.session.action","kind":"action","label":"Visual-only semantic action","fontSize":16,"enabled":true,"tone":"accent"}]}}';

/** The semantic action that ends the field diagnostic. */
export const FIELD_SESSION_ACTION = "sample.fields.submit";

/** The field IDs the field diagnostic expects to read back. */
export const FIELD_SESSION_IDS = ["sample.fields.name", "sample.fields.note"] as const;

/**
 * The document used to verify that typing reaches an application only on request.
 *
 * The application supplies each field's starting value once, here. What a
 * person types afterwards lives in the host, and arrives only through an
 * explicit `ui.fields.read` — there is no change event to subscribe to. One
 * field starts pre-filled so the diagnostic can show a value being *edited*
 * rather than only entered.
 */
export const FIELD_SESSION_DOCUMENT = JSON.stringify({
  format: "anodrel.ui.document.v1",
  root: {
    id: "sample.fields.root",
    kind: "stack",
    axis: "vertical",
    padding: { left: 56, top: 48, right: 56, bottom: 48 },
    gap: 14,
    surfaceTone: "plain",
    children: [
      {
        id: "sample.fields.eyebrow",
        kind: "text",
        value: "AUTHENTICATED ANODREL SESSION",
        fontSize: 14,
        tone: "accent",
      },
      {
        id: "sample.fields.title",
        kind: "text",
        value: "Type something, then submit",
        fontSize: 28,
        tone: "primary",
      },
      {
        id: "sample.fields.detail",
        kind: "text",
        value:
          "Nothing you type reaches this application until you activate the action below. There is no change event to subscribe to.",
        fontSize: 16,
        tone: "secondary",
      },
      {
        id: FIELD_SESSION_IDS[0],
        kind: "field",
        label: "Name",
        value: "",
        placeholder: "Tab here and type",
        maxLength: 64,
        fontSize: 16,
        enabled: true,
      },
      {
        id: FIELD_SESSION_IDS[1],
        kind: "field",
        label: "Note",
        value: "edit me",
        maxLength: 64,
        fontSize: 16,
        enabled: true,
      },
      {
        id: FIELD_SESSION_ACTION,
        kind: "action",
        label: "Submit field values",
        fontSize: 16,
        enabled: true,
        tone: "accent",
      },
    ],
  },
});

/**
 * Builds the document this application publishes after reading field values.
 *
 * The point of echoing them back into the window is that the host suppresses
 * the sample child's console output on purpose, so a value printed there is
 * invisible. Publishing it proves the application really received the text,
 * on the one surface a person is already looking at.
 *
 * An empty value is shown as `(empty)` because a text node must carry text —
 * and because "you left this blank" is itself worth seeing.
 */
export function fieldEchoDocument(
  values: ReadonlyArray<{ readonly id: string; readonly value: string }>,
): string {
  return JSON.stringify({
    format: "anodrel.ui.document.v1",
    root: {
      id: "sample.fields.echo.root",
      kind: "stack",
      axis: "vertical",
      padding: { left: 56, top: 48, right: 56, bottom: 48 },
      gap: 14,
      surfaceTone: "plain",
      children: [
        {
          id: "sample.fields.echo.eyebrow",
          kind: "text",
          value: "RECEIVED BY THE APPLICATION",
          fontSize: 14,
          tone: "accent",
        },
        {
          id: "sample.fields.echo.title",
          kind: "text",
          value: "This arrived once, because you asked",
          fontSize: 28,
          tone: "primary",
        },
        ...values.map((entry, index) => ({
          id: `sample.fields.echo.value.${index}`,
          kind: "text" as const,
          value: `${entry.id} = ${entry.value === "" ? "(empty)" : entry.value}`,
          fontSize: 16,
          tone: "primary" as const,
        })),
        // Deliberately one long sentence. An earlier run of this document was
        // cut off mid-word at the window edge, because a text run did not wrap;
        // it now reflows to the column, so this line is also the check that it
        // still does. See `docs/UI.md`.
        {
          id: "sample.fields.echo.detail",
          kind: "text",
          value:
            "No keystroke, caret, or timing crossed the boundary. Everything typed before you submitted happened without this application ever watching.",
          fontSize: 16,
          tone: "secondary",
        },
      ],
    },
  });
}

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
