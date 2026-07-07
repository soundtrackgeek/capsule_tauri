import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type RefObject,
} from "react";
import "../../lib/retro-focus/styles.css";
import {
  resolveRetroFocusEditorStyle,
  type RetroFocusThemeCustomization,
} from "../../lib/retro-focus/customization";
import { renderThemeContent } from "../../lib/retro-focus/themeRenderers";
import {
  RETRO_FOCUS_THEMES_BY_ID,
  type RetroFocusThemeDefinition,
  type RetroFocusThemeId,
} from "../../lib/retro-focus/themes";

type CursorMetrics = {
  line: number;
  col: number;
};

type ThemeVars = CSSProperties & Record<string, string>;

export type RetroWriterShellHandle = {
  focus: () => boolean;
};

type RetroWriterShellProps = {
  text: string;
  themeId: RetroFocusThemeId;
  customization?: RetroFocusThemeCustomization;
  onChange: (value: string) => void;
  autoFocus?: boolean;
  className?: string;
  style?: CSSProperties;
  ariaLabel?: string;
};

function getCursorMetrics(value: string, selectionStart: number): CursorMetrics {
  const cappedStart = Math.max(0, Math.min(selectionStart, value.length));
  const before = value.slice(0, cappedStart);
  const lines = before.split("\n");
  const line = lines.length;
  const col = (lines[lines.length - 1]?.length ?? 0) + 1;

  return { line, col };
}

function buildThemeVars(
  theme: RetroFocusThemeDefinition,
  customization?: RetroFocusThemeCustomization,
): ThemeVars {
  const gutter = theme.editorLayout.gutter;
  const screenLayout = theme.screenLayout;
  const editorStyle = resolveRetroFocusEditorStyle(theme, customization);

  return {
    "--layout-scale": screenLayout?.scale ?? "1",
    "--layout-safe-inset": screenLayout?.safeInset ?? "0",
    "--app-background": theme.screenFx.appBackground,
    "--room-glow": theme.screenFx.roomGlow,
    "--room-grid-major": theme.screenFx.roomGridMajor,
    "--room-grid-minor": theme.screenFx.roomGridMinor,
    "--room-grid-opacity": theme.screenFx.roomGridOpacity,
    "--frame-shadow": theme.screenFx.frameShadow,
    "--monitor-underlay": theme.screenFx.monitorUnderlay,
    "--screen-inset": "0px",
    "--screen-background": theme.screenFx.screenBackground,
    "--screen-tint": theme.screenFx.screenTint,
    "--screen-vignette": theme.screenFx.screenVignette,
    "--screen-inner-shadow": theme.screenFx.screenInnerShadow,
    "--scanline-image": theme.screenFx.scanline,
    "--scanline-opacity": theme.screenFx.scanlineOpacity,
    "--noise-opacity": theme.screenFx.noiseOpacity,
    "--text-glow": theme.screenFx.textGlow,
    "--content-padding-top": theme.screenFx.contentPaddingTop,
    "--content-padding-right": theme.screenFx.contentPaddingRight,
    "--content-padding-bottom": theme.screenFx.contentPaddingBottom,
    "--content-padding-left": theme.screenFx.contentPaddingLeft,
    "--text-color": theme.chrome.textColor,
    "--accent-color": theme.chrome.accentColor,
    "--dim-text-color": theme.chrome.dimTextColor,
    "--divider-color": theme.chrome.dividerColor,
    "--inverse-text-color": theme.chrome.inverseTextColor ?? theme.chrome.textColor,
    "--inverse-background": theme.chrome.inverseBackground ?? theme.chrome.accentColor,
    "--editor-font-family": theme.editorLayout.fontFamily,
    "--editor-font-size": theme.editorLayout.fontSize,
    "--editor-ui-font-family": editorStyle.fontFamily,
    "--editor-ui-font-size": editorStyle.fontSize,
    "--editor-ui-text-color": editorStyle.textColor,
    "--editor-ui-prompt-color": editorStyle.promptColor,
    "--editor-ui-gutter-color": editorStyle.gutterColor,
    "--editor-ui-gutter-active-color": editorStyle.gutterActiveColor,
    "--editor-ui-gutter-font-size":
      customization?.fontSizePx !== undefined
        ? editorStyle.fontSize
        : gutter?.fontSize ?? theme.editorLayout.fontSize,
    "--editor-ui-placeholder-color": editorStyle.placeholderColor,
    "--editor-line-height": theme.editorLayout.lineHeight,
    "--editor-letter-spacing": theme.editorLayout.letterSpacing,
    "--editor-font-weight": theme.editorLayout.fontWeight,
    "--editor-padding-top": theme.editorLayout.paddingTop,
    "--editor-padding-right": theme.editorLayout.paddingRight,
    "--editor-padding-bottom": theme.editorLayout.paddingBottom,
    "--editor-padding-left": theme.editorLayout.paddingLeft,
    "--editor-caret-color": theme.editorLayout.caretColor,
    "--selection-color": theme.editorLayout.selectionColor,
    "--prompt-color": theme.editorLayout.promptColor ?? theme.chrome.accentColor,
    "--gutter-width": gutter?.width ?? "0px",
    "--gutter-gap": gutter?.gap ?? "0px",
    "--gutter-color": gutter?.color ?? theme.chrome.dimTextColor,
    "--gutter-active-color": gutter?.activeColor ?? theme.chrome.accentColor,
    "--gutter-font-size": gutter?.fontSize ?? theme.editorLayout.fontSize,
  };
}

type EditorPaneProps = {
  cursor: CursorMetrics;
  onChange: (value: string) => void;
  onScroll: (scrollTop: number) => void;
  onSelect: (selectionStart: number) => void;
  scrollTop: number;
  text: string;
  theme: RetroFocusThemeDefinition;
  textareaRef: RefObject<HTMLTextAreaElement | null>;
};

function EditorPane({
  cursor,
  onChange,
  onScroll,
  onSelect,
  scrollTop,
  text,
  theme,
  textareaRef,
}: EditorPaneProps) {
  const gutter = theme.editorLayout.gutter;
  const showPrompt = Boolean(theme.editorLayout.prompt) && scrollTop < 1;
  const gutterLines = gutter
    ? Array.from({ length: gutter.lineCount }, (_, index) => {
        const lineNumber = index + 1;
        return {
          lineNumber,
          label: gutter.formatter
            ? gutter.formatter(lineNumber)
            : `${lineNumber.toString().padStart(2, " ")}:`,
        };
      })
    : [];

  const handleSelection = useCallback(() => {
    const textarea = textareaRef.current;
    if (!textarea) {
      return;
    }

    onSelect(textarea.selectionStart);
  }, [onSelect, textareaRef]);

  return (
    <div
      className={[
        "editor-pane",
        gutter ? "editor-pane--with-gutter" : "",
        showPrompt ? "editor-pane--with-prompt" : "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {gutter ? (
        <div className="editor-pane__gutter" aria-hidden="true">
          <div
            className="editor-pane__gutter-inner"
            style={{ transform: `translateY(${-scrollTop}px)` }}
          >
            {gutterLines.map(({ lineNumber, label }) => (
              <span
                key={lineNumber}
                className={[
                  "editor-pane__gutter-line",
                  cursor.line === lineNumber ? "editor-pane__gutter-line--active" : "",
                ]
                  .filter(Boolean)
                  .join(" ")}
              >
                {label}
              </span>
            ))}
          </div>
        </div>
      ) : null}

      <div className="editor-pane__field">
        {showPrompt ? (
          <span className="editor-pane__prompt" aria-hidden="true">
            {theme.editorLayout.prompt}
          </span>
        ) : null}
        <textarea
          ref={textareaRef}
          value={text}
          onChange={(event) => {
            onChange(event.target.value);
            onSelect(event.currentTarget.selectionStart);
          }}
          onClick={handleSelection}
          onKeyUp={handleSelection}
          onScroll={(event) => onScroll(event.currentTarget.scrollTop)}
          onSelect={(event) => onSelect(event.currentTarget.selectionStart)}
          spellCheck={false}
          autoCapitalize="off"
          autoComplete="off"
          autoCorrect="off"
          placeholder={showPrompt ? "" : theme.editorLayout.placeholder}
          className="editor-pane__input"
        />
      </div>
    </div>
  );
}

export const RetroWriterShell = forwardRef<RetroWriterShellHandle, RetroWriterShellProps>(
  function RetroWriterShell(
    { text, themeId, customization, onChange, autoFocus = false, className, style, ariaLabel },
    forwardedRef,
  ) {
    const textareaRef = useRef<HTMLTextAreaElement | null>(null);
    const [cursor, setCursor] = useState<CursorMetrics>({ line: 1, col: 1 });
    const [now, setNow] = useState(() => new Date());
    const [scrollTop, setScrollTop] = useState(0);
    const theme = RETRO_FOCUS_THEMES_BY_ID[themeId];
    const themeVars = useMemo(
      () => buildThemeVars(theme, customization),
      [customization, theme],
    );

    useEffect(() => {
      const timer = window.setInterval(() => setNow(new Date()), 30_000);
      return () => window.clearInterval(timer);
    }, []);

    useEffect(() => {
      if (!autoFocus) {
        return;
      }

      const focusTimer = window.setTimeout(() => textareaRef.current?.focus(), 0);
      return () => window.clearTimeout(focusTimer);
    }, [autoFocus, themeId]);

    const handleSelect = useCallback(
      (selectionStart: number) => {
        setCursor(getCursorMetrics(text, selectionStart));
      },
      [text],
    );

    useImperativeHandle(
      forwardedRef,
      () => ({
        focus: () => {
          const textarea = textareaRef.current;
          if (!textarea) {
            return false;
          }

          textarea.focus();
          return true;
        },
      }),
      [],
    );

    return (
      <div
        className={["retro-app-shell", "retro-focus-app-shell--transparent", className]
          .filter(Boolean)
          .join(" ")}
        style={{ ...themeVars, ...style }}
      >
        <div className="retro-app-stage retro-focus-stage--plain">
          <section
            className="monitor-shell monitor-shell--frameless"
            aria-label={ariaLabel ?? `Retro Writer in ${theme.label} theme`}
          >
            <div className={`theme-screen theme-screen--${theme.chrome.variant}`}>
              <div className="theme-screen__tint" />
              <div className="theme-screen__vignette" />
              <div className="theme-screen__scanlines" />
              <div className="theme-screen__noise" />

              <div className="theme-screen__viewport">
                <div className="theme-screen__canvas">
                  <div className="theme-screen__content">
                    {renderThemeContent({
                      cursor,
                      editorPane: (
                        <EditorPane
                          cursor={cursor}
                          onChange={onChange}
                          onScroll={setScrollTop}
                          onSelect={handleSelect}
                          scrollTop={scrollTop}
                          text={text}
                          theme={theme}
                          textareaRef={textareaRef}
                        />
                      ),
                      now,
                      textLength: text.length,
                      theme,
                    })}
                  </div>
                </div>
              </div>
            </div>
          </section>
        </div>
      </div>
    );
  },
);
