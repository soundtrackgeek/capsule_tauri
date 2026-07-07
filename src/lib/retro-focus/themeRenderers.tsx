/* eslint-disable react-refresh/only-export-components */
import type { CSSProperties, ReactElement, ReactNode } from 'react';
import type {
  LcarsRailSegment,
  LcarsTone,
  LcarsToneDefinition,
  ThemeChrome,
  ThemeDefinition,
  ThemeVariant,
} from './themes';

type CursorMetrics = {
  line: number;
  col: number;
};

export type ThemeRendererProps = {
  cursor: CursorMetrics;
  editorPane: ReactNode;
  now: Date;
  textLength: number;
  theme: ThemeDefinition;
};

type ThemeRenderer = (props: ThemeRendererProps) => ReactElement | null;
type ToneMap = Partial<Record<LcarsTone, LcarsToneDefinition>>;

function formatTime(date: Date) {
  return new Intl.DateTimeFormat(undefined, {
    hour: 'numeric',
    minute: '2-digit',
  }).format(date);
}

function formatTwoDigits(value: number) {
  return value.toString().padStart(2, '0');
}

function formatThreeDigits(value: number) {
  return value.toString().padStart(3, '0');
}

function formatTwentyFourHourTime(date: Date) {
  return `${formatTwoDigits(date.getHours())}:${formatTwoDigits(date.getMinutes())}:${formatTwoDigits(date.getSeconds())}`;
}

function formatDateTimeStamp(date: Date) {
  return [
    date.getFullYear(),
    formatTwoDigits(date.getMonth() + 1),
    formatTwoDigits(date.getDate()),
  ].join('-') + ` ${formatTwentyFourHourTime(date)}`;
}

function replaceMetricTokens(text: string, cursor: CursorMetrics) {
  return text
    .replace(/LN:\s*\d+/gi, `LN:${formatTwoDigits(cursor.line)}`)
    .replace(/COL:\s*\d+/gi, `COL:${formatTwoDigits(cursor.col)}`)
    .replace(/LINE:\s*\d+/gi, `LINE: ${cursor.line}`)
    .replace(/\bLINE\s+\d+\b/gi, `LINE ${cursor.line}`)
    .replace(/\bCOL\s+\d+\b/gi, `COL ${cursor.col}`);
}

function getThemeChrome<TVariant extends ThemeVariant>(
  theme: ThemeDefinition,
  variant: TVariant,
): Extract<ThemeChrome, { variant: TVariant }> | null {
  return theme.chrome.variant === variant
    ? (theme.chrome as Extract<ThemeChrome, { variant: TVariant }>)
    : null;
}

function Legend({ lines }: { lines: string[][] }) {
  return (
    <div className="theme-legend" aria-hidden="true">
      {lines.map((line, lineIndex) => (
        <div key={lineIndex} className="theme-legend__row">
          {line.map((item, itemIndex) => (
            <span
              key={`${lineIndex}-${itemIndex}`}
              className={[
                'theme-legend__item',
                item ? '' : 'theme-legend__item--empty',
              ]
                .filter(Boolean)
                .join(' ')}
            >
              {item || ' '}
            </span>
          ))}
        </div>
      ))}
    </div>
  );
}

function getToneStyle(tones: ToneMap, tone: LcarsTone): CSSProperties {
  const toneValue = tones[tone];

  return {
    background: toneValue?.background ?? '#f1b36b',
    color: toneValue?.textColor ?? '#101427',
  };
}

function LcarsActionPill({
  action,
  tones,
  className,
}: {
  action: { label: string; tone: LcarsTone };
  tones: ToneMap;
  className?: string;
}) {
  return (
    <span
      className={['lcars-pill', className].filter(Boolean).join(' ')}
      style={getToneStyle(tones, action.tone)}
    >
      {action.label}
    </span>
  );
}

function LcarsRail({
  className,
  segments,
  tones,
}: {
  className?: string;
  segments: LcarsRailSegment[];
  tones: ToneMap;
}) {
  return (
    <div className={['lcars-rail', className].filter(Boolean).join(' ')}>
      {segments.map((segment, index) => (
        <span
          key={`${segment.tone}-${index}`}
          className="lcars-rail__segment"
          style={{
            ...getToneStyle(tones, segment.tone),
            flex: `${segment.flex ?? 1} 1 0`,
          }}
        >
          {segment.label ?? ''}
        </span>
      ))}
    </div>
  );
}

function ClassicThemeRenderer({
  cursor,
  editorPane,
  now,
  theme,
}: ThemeRendererProps) {
  const chrome = getThemeChrome(theme, 'classic');

  if (!chrome) {
    return null;
  }

  return (
    <>
      <div className="classic-screen__menu">
        {chrome.menuItems.map((item) => (
          <span key={item}>{item}</span>
        ))}
      </div>

      <div className="classic-screen__path">{chrome.pathText}</div>

      <div className="classic-screen__body">
        <div className="classic-screen__title">{chrome.documentTitle}</div>
        <div className="classic-screen__title-rule" />
        <div className="classic-screen__editor">{editorPane}</div>
      </div>

      <div className="classic-screen__footer">
        <div className="classic-screen__footer-metrics">
          <span>Page: 1</span>
          <span>Line: {cursor.line}</span>
          <span>Col: {cursor.col}</span>
          <span>Mode: {chrome.modeLabel}</span>
        </div>
        <div>{formatTime(now)}</div>
      </div>
    </>
  );
}

function StatusBarThemeRenderer({
  cursor,
  editorPane,
  theme,
}: ThemeRendererProps) {
  const chrome = getThemeChrome(theme, 'status-bar');

  if (!chrome) {
    return null;
  }

  return (
    <>
      <div className="status-bar-screen__header">
        <span>File: {chrome.fileName}</span>
        <span>| Ln: {cursor.line}</span>
        <span>| Col: {cursor.col}</span>
        <span>| {chrome.modeLabel}</span>
        <span>| {chrome.directoryLabel}</span>
        <span>| {chrome.menuLabel}</span>
      </div>

      <div className="status-bar-screen__editor">{editorPane}</div>
    </>
  );
}

function RulerThemeRenderer({
  cursor,
  editorPane,
  theme,
}: ThemeRendererProps) {
  const chrome = getThemeChrome(theme, 'ruler');

  if (!chrome) {
    return null;
  }

  return (
    <>
      <div className="ruler-screen__header">
        <span>{`** ${chrome.modeLabel} **`}</span>
        <span>{`[ File: ${chrome.fileName} ]`}</span>
        <span>{`Line: ${cursor.line}  Col: ${cursor.col}`}</span>
      </div>

      <div className="ruler-screen__rule">{chrome.rulerText}</div>

      <div className="ruler-screen__editor">{editorPane}</div>

      {theme.legend ? <Legend lines={theme.legend.lines} /> : null}
    </>
  );
}

function BracketedThemeRenderer({
  cursor,
  editorPane,
  theme,
}: ThemeRendererProps) {
  const chrome = getThemeChrome(theme, 'bracketed');

  if (!chrome) {
    return null;
  }

  return (
    <>
      <div className="bracketed-screen__header">
        <span>{`[FILE: ${chrome.fileName} ]`}</span>
        <span>{`[MODE: ${chrome.modeLabel} ]`}</span>
        <span>{`[ Ln ${cursor.line}, Col ${cursor.col} ]`}</span>
        <span>{`[ ${chrome.insertLabel} ]`}</span>
      </div>

      <div className="theme-divider" />

      <div className="bracketed-screen__editor">{editorPane}</div>
    </>
  );
}

function WritingThemeRenderer({
  cursor,
  editorPane,
  theme,
}: ThemeRendererProps) {
  const chrome = getThemeChrome(theme, 'writing');

  if (!chrome) {
    return null;
  }

  return (
    <>
      <div className="writing-screen__header">
        <span>{`FILE: [${chrome.fileName}]`}</span>
        <span>{`| L${formatTwoDigits(cursor.line)} C${formatTwoDigits(cursor.col)}`}</span>
        <span>{`| PAGE: ${chrome.pageLabel}`}</span>
        <span>{`| MODE: ${chrome.modeLabel}`}</span>
        <span>{`| CMD: ${chrome.commandField}`}</span>
      </div>

      <div className="theme-divider" />

      <div className="writing-screen__editor">{editorPane}</div>

      {theme.legend ? <Legend lines={theme.legend.lines} /> : null}
    </>
  );
}

function AppleworksMenuThemeRenderer({
  cursor,
  editorPane,
  theme,
}: ThemeRendererProps) {
  const chrome = getThemeChrome(theme, 'appleworks-menu');

  if (!chrome) {
    return null;
  }

  return (
    <div className="appleworks-screen">
      <div className="appleworks-screen__masthead">
        <span>{chrome.mastheadLeft}</span>
        <span className="appleworks-screen__masthead-center">
          {chrome.mastheadCenter}
        </span>
        <span className="appleworks-screen__masthead-right">
          {chrome.mastheadRight}
        </span>
      </div>

      <div className="appleworks-screen__menu">
        {chrome.menuItems.map((item) => (
          <span key={item}>{item}</span>
        ))}
      </div>

      <div className="appleworks-screen__status">
        <span>{chrome.fileLabel}</span>
        <span>{chrome.pageLabel}</span>
        <span>{`LINE  ${cursor.line}`}</span>
      </div>

      <div className="appleworks-screen__editor">{editorPane}</div>
    </div>
  );
}

function ApplewriterFamilyThemeRenderer({
  cursor,
  editorPane,
  theme,
}: ThemeRendererProps) {
  const chrome = getThemeChrome(theme, 'applewriter-family');

  if (!chrome) {
    return null;
  }

  return (
    <div className="applewriter-screen">
      <div
        className={[
          'applewriter-screen__title',
          `applewriter-screen__title--${chrome.titleAlign ?? 'left'}`,
        ].join(' ')}
      >
        {replaceMetricTokens(chrome.titleLine, cursor)}
      </div>

      {chrome.statusLine ? (
        <div
          className={[
            'applewriter-screen__status',
            `applewriter-screen__status--${chrome.statusAlign ?? 'left'}`,
          ].join(' ')}
        >
          {replaceMetricTokens(chrome.statusLine, cursor)}
        </div>
      ) : null}

      <div className="applewriter-screen__editor">{editorPane}</div>
    </div>
  );
}

function BannerFooterThemeRenderer({
  cursor,
  editorPane,
  theme,
}: ThemeRendererProps) {
  const chrome = getThemeChrome(theme, 'banner-footer');

  if (!chrome) {
    return null;
  }

  return (
    <div
      className={[
        'banner-footer',
        chrome.editorBordered ? 'banner-footer--bordered' : '',
      ]
        .filter(Boolean)
        .join(' ')}
    >
      <div className="banner-footer__header">
        {chrome.headerLines.map((line, index) => (
          <div
            key={`${line.text}-${index}`}
            className={[
              'banner-footer__line',
              line.inverse ? 'banner-footer__line--inverse' : '',
              `banner-footer__line--${line.align ?? 'left'}`,
            ]
              .filter(Boolean)
              .join(' ')}
          >
            {replaceMetricTokens(line.text, cursor)}
          </div>
        ))}
      </div>

      {chrome.dividerAfterHeader ? <div className="theme-divider" /> : null}

      <div className="banner-footer__editor-shell">
        <div className="banner-footer__editor">{editorPane}</div>
      </div>

      {chrome.dividerBeforeFooter ? <div className="theme-divider" /> : null}

      <div className="banner-footer__footer">
        {chrome.footerLines.map((line, index) => (
          <div
            key={`${line.text}-${index}`}
            className={[
              'banner-footer__line',
              line.inverse ? 'banner-footer__line--inverse' : '',
              `banner-footer__line--${line.align ?? 'left'}`,
            ]
              .filter(Boolean)
              .join(' ')}
          >
            {replaceMetricTokens(line.text, cursor)}
          </div>
        ))}
      </div>
    </div>
  );
}

function FramedTerminalThemeRenderer({
  cursor,
  editorPane,
  theme,
}: ThemeRendererProps) {
  const chrome = getThemeChrome(theme, 'framed-terminal');

  if (!chrome) {
    return null;
  }

  return (
    <div className="framed-terminal">
      <div className="framed-terminal__header">
        <span>{chrome.headerLeft}</span>
        <span>{chrome.headerCenter}</span>
        <span>{replaceMetricTokens(chrome.headerRight, cursor)}</span>
      </div>

      <div className="framed-terminal__editor-shell">
        <div className="framed-terminal__editor">{editorPane}</div>
      </div>

      <div className="framed-terminal__status-row">
        <span className="framed-terminal__status-pill">{chrome.statusLabel}</span>
      </div>

      <div className="framed-terminal__commands">
        {chrome.commandHints.map((hint) => (
          <span key={hint}>{hint}</span>
        ))}
      </div>
    </div>
  );
}

function UnixStatusThemeRenderer({
  cursor,
  editorPane,
  theme,
}: ThemeRendererProps) {
  const chrome = getThemeChrome(theme, 'unix-status');

  if (!chrome) {
    return null;
  }

  return (
    <div className="unix-status-screen">
      <div className="unix-status-screen__title">
        {replaceMetricTokens(chrome.titleLine, cursor)}
      </div>

      <div className="unix-status-screen__meta">{chrome.metaLine}</div>

      <div className="unix-status-screen__editor">{editorPane}</div>

      <div className="unix-status-screen__footer">
        <span>{chrome.footerLeft}</span>
        <span>{replaceMetricTokens(chrome.footerRight, cursor)}</span>
      </div>
    </div>
  );
}

function StarfleetCommandThemeRenderer({
  editorPane,
  theme,
}: ThemeRendererProps) {
  const chrome = getThemeChrome(theme, 'starfleet-command');

  if (!chrome) {
    return null;
  }

  return (
    <div className="lcars-command">
      <div className="lcars-command__masthead">
        <div className="lcars-command__seal" aria-hidden="true">
          <span className="lcars-command__seal-core" />
        </div>

        <div className="lcars-command__masthead-copy">
          <span>{chrome.mastheadLines[0]}</span>
          <span>{chrome.mastheadLines[1]}</span>
        </div>
      </div>

      <div className="lcars-command__title-row">
        <div className="lcars-command__title-shell">
          <div className="lcars-command__title-notch" />
          <span className="lcars-command__title-label">{chrome.title}</span>
        </div>

        <div className="lcars-command__top-actions">
          {chrome.topActions.map((action) => (
            <LcarsActionPill
              key={action.label}
              action={action}
              tones={chrome.tones}
              className="lcars-command__top-pill"
            />
          ))}
        </div>
      </div>

      <div className="lcars-command__rails">
        <LcarsRail
          className="lcars-command__rail lcars-command__rail--upper"
          segments={chrome.upperRails}
          tones={chrome.tones}
        />
        <LcarsRail
          className="lcars-command__rail lcars-command__rail--lower"
          segments={chrome.lowerRails}
          tones={chrome.tones}
        />
      </div>

      <div className="lcars-command__body">
        <div className="lcars-command__action-column">
          {chrome.leftActions.map((action) => (
            <LcarsActionPill
              key={action.label}
              action={action}
              tones={chrome.tones}
              className="lcars-command__side-pill"
            />
          ))}
        </div>

        <div className="lcars-command__workspace">
          <div className="lcars-command__banner">
            <span>{`CURRENT DOCUMENT: ${chrome.documentName}`}</span>
            <span>{`STATUS: ${chrome.statusText}`}</span>
          </div>

          <div className="lcars-command__editor-frame">
            <div className="lcars-command__prompt-line">
              <span>{chrome.editorHeading}</span>
              <span className="lcars-command__cursor-block" />
            </div>
            <div className="lcars-command__editor">{editorPane}</div>
          </div>

          <div className="lcars-command__footer">
            {chrome.footerFields.map((field) => (
              <span key={field}>{field}</span>
            ))}
          </div>
        </div>

        <div className="lcars-command__status-column">
          {chrome.rightPanels.map((panel) => (
            <LcarsActionPill
              key={panel.label}
              action={panel}
              tones={chrome.tones}
              className="lcars-command__status-pill"
            />
          ))}
        </div>
      </div>
    </div>
  );
}

function EnterpriseSystemsGlyph() {
  return (
    <div className="lcars-enterprise__glyph" aria-hidden="true">
      <span className="lcars-enterprise__glyph-outer" />
      <span className="lcars-enterprise__glyph-inner" />
      <span className="lcars-enterprise__glyph-core" />
    </div>
  );
}

function EnterpriseSystemsThemeRenderer({
  editorPane,
  now,
  textLength,
  theme,
}: ThemeRendererProps) {
  const chrome = getThemeChrome(theme, 'enterprise-systems');

  if (!chrome) {
    return null;
  }

  return (
    <div className="lcars-enterprise">
      <div className="lcars-enterprise__header">
        <div className="lcars-enterprise__header-shell lcars-enterprise__header-shell--left">
          <span>{chrome.headerLeft}</span>
        </div>
        <div className="lcars-enterprise__header-shell lcars-enterprise__header-shell--right">
          <span>{chrome.headerRight}</span>
        </div>
      </div>

      <div className="lcars-enterprise__subtitle-row">
        <span>{chrome.subtitle}</span>
        <div className="lcars-enterprise__subtitle-rails">
          <LcarsRail
            className="lcars-enterprise__subtitle-rail"
            segments={chrome.upperRails}
            tones={chrome.tones}
          />
          <LcarsRail
            className="lcars-enterprise__subtitle-rail lcars-enterprise__subtitle-rail--lower"
            segments={chrome.lowerRails}
            tones={chrome.tones}
          />
        </div>
      </div>

      <div className="lcars-enterprise__layout">
        <div className="lcars-enterprise__left-column">
          <EnterpriseSystemsGlyph />

          <div className="lcars-enterprise__left-actions">
            {chrome.primaryActions.map((action) => (
              <LcarsActionPill
                key={action.label}
                action={action}
                tones={chrome.tones}
                className="lcars-enterprise__action-pill"
              />
            ))}
          </div>

          <div className="lcars-enterprise__left-actions lcars-enterprise__left-actions--secondary">
            {chrome.secondaryActions.map((action) => (
              <LcarsActionPill
                key={action.label}
                action={action}
                tones={chrome.tones}
                className="lcars-enterprise__action-pill"
              />
            ))}
          </div>
        </div>

        <div className="lcars-enterprise__workspace">
          <div className="lcars-enterprise__status-card">
            <div className="lcars-enterprise__status-column">
              <span>STATUS: READY</span>
              <span>CHAPTERS:</span>
            </div>

            <div className="lcars-enterprise__status-column">
              <span>{`DOCUMENT: ${chrome.documentName}`}</span>
              <span>{`DATE: STARDATE ${chrome.stardate}`}</span>
              <span>{`CHARACTERS: ${textLength}`}</span>
            </div>
          </div>

          <div className="lcars-enterprise__editor-frame">
            <div className="lcars-enterprise__editor">{editorPane}</div>
          </div>

          <div className="lcars-enterprise__menu-rail">
            {chrome.footerMenu.map((menuItem) => (
              <span key={menuItem}>{menuItem}</span>
            ))}
          </div>

          <div className="lcars-enterprise__footer">
            <span>{chrome.loggedInAs}</span>
            <span>{chrome.terminal}</span>
            <span>{chrome.securityText}</span>
            <span>{`TIME: ${formatTwentyFourHourTime(now)} ${chrome.timeZone}`}</span>
          </div>
        </div>
      </div>
    </div>
  );
}

function Lcars41ThemeRenderer({
  cursor,
  editorPane,
  theme,
}: ThemeRendererProps) {
  const chrome = getThemeChrome(theme, 'lcars-4-1');

  if (!chrome) {
    return null;
  }

  return (
    <div className="lcars-41">
      <div className="lcars-41__title">{chrome.title}</div>
      <div className="lcars-41__command-line">{chrome.commandLine}</div>

      <div className="lcars-41__meta-line">
        {chrome.metaFields.map((field, index) => (
          <span key={field}>
            {index > 0 ? ' / ' : ''}
            {field}
          </span>
        ))}
      </div>

      <div className="lcars-41__status-strip">
        <span>{chrome.systemStatus}</span>
        <div className="lcars-41__status-bar" />
        <span>{chrome.networkStatus}</span>
      </div>

      <div className="lcars-41__layout">
        <div className="lcars-41__side-column lcars-41__side-column--left">
          {chrome.leftActions.map((action) => (
            <LcarsActionPill
              key={action.label}
              action={action}
              tones={chrome.tones}
              className="lcars-41__side-pill"
            />
          ))}
        </div>

        <div className="lcars-41__workspace">
          <div className="lcars-41__editor-shell">
            <div className="lcars-41__editor-title">{chrome.editorTitle}</div>
            <div className="lcars-41__editor">{editorPane}</div>
            <div className="lcars-41__measure-strip" aria-hidden="true">
              {Array.from({ length: 30 }, (_, index) => index + 1).map((value) => (
                <span key={value}>{formatTwoDigits(value)}</span>
              ))}
            </div>
            <div className="lcars-41__editor-footer">
              <span>{`[CURSOR: L${cursor.line} | C${cursor.col}]`}</span>
              <span>{chrome.fileFooter}</span>
            </div>
          </div>
        </div>

        <div className="lcars-41__side-column lcars-41__side-column--right">
          {chrome.rightActions.map((action) => (
            <LcarsActionPill
              key={action.label}
              action={action}
              tones={chrome.tones}
              className="lcars-41__side-pill"
            />
          ))}
        </div>
      </div>

      <div className="lcars-41__toolbar">
        {chrome.bottomActions.map((action) => (
          <LcarsActionPill
            key={action.label}
            action={action}
            tones={chrome.tones}
            className="lcars-41__toolbar-pill"
          />
        ))}
      </div>
    </div>
  );
}

function Mother6800ThemeRenderer({
  cursor,
  editorPane,
  theme,
}: ThemeRendererProps) {
  const chrome = getThemeChrome(theme, 'mother-6800');

  if (!chrome) {
    return null;
  }

  return (
    <div className="alien-terminal alien-mother">
      <div className="alien-mother__masthead">
        <div className="alien-mother__title-block">
          <span className="alien-mother__system">{chrome.systemName}</span>
          <span className="alien-mother__console">{chrome.consoleName}</span>
        </div>

        <div className="alien-mother__status-grid">
          <span>{`FILE: ${chrome.documentName}`}</span>
          <span>{`PAGE: ${chrome.pageLabel}`}</span>
          <span>{`LINE: ${formatThreeDigits(cursor.line)}`}</span>
          <span>{`COL: ${formatThreeDigits(cursor.col)}`}</span>
          <span>{`MODE: ${chrome.modeLabel}`}</span>
        </div>
      </div>

      <div className="alien-mother__menu">
        {chrome.menuItems.map((item) => (
          <span key={item}>{item}</span>
        ))}
      </div>

      <div className="alien-mother__workspace">
        <div className="alien-mother__editor-shell">
          <div className="alien-mother__prompt">{chrome.promptLabel}</div>
          <div className="alien-mother__editor">{editorPane}</div>
        </div>

        <div className="alien-mother__rail" aria-hidden="true">
          {chrome.railItems.map((item) => (
            <span key={item}>{item}</span>
          ))}
        </div>
      </div>

      <div className="alien-mother__footer">
        <span>{chrome.footerLeft}</span>
        <span>{chrome.footerRight}</span>
      </div>
    </div>
  );
}

function ReadyInputThemeRenderer({ editorPane, theme }: ThemeRendererProps) {
  const chrome = getThemeChrome(theme, 'ready-input');

  if (!chrome) {
    return null;
  }

  return (
    <div className="alien-terminal alien-ready">
      <div className="alien-ready__header">
        <span>{`~DOCUMENT: ${chrome.documentName}`}</span>
        <span>{`STATUS: ${chrome.statusLabel}`}</span>
        <span>{`PAGE ${chrome.pageLabel}`}</span>
      </div>

      <div className="alien-ready__status-line">{chrome.statusLabel}</div>

      <div className="alien-ready__editor-shell">
        <div className="alien-ready__editor">{editorPane}</div>
      </div>

      <div className="alien-ready__command-strip">
        {chrome.commandHints.map((hint) => (
          <span key={hint}>{hint}</span>
        ))}
      </div>

      <div className="alien-ready__footer">{chrome.footerHint}</div>
    </div>
  );
}

function NostromoDataSystemsThemeRenderer({
  cursor,
  editorPane,
  now,
  theme,
}: ThemeRendererProps) {
  const chrome = getThemeChrome(theme, 'nostromo-data-systems');

  if (!chrome) {
    return null;
  }

  return (
    <div className="alien-terminal alien-nostromo">
      <div className="alien-nostromo__header">
        <span className="alien-nostromo__title">{chrome.headerTitle}</span>
        <span className="alien-nostromo__access">{chrome.accessLabel}</span>
      </div>

      <div className="alien-nostromo__subheader">
        <span>{chrome.editorStatus}</span>
        <span>{`[DOCUMENT: ${chrome.documentName}]`}</span>
      </div>

      <div className="alien-nostromo__status-line">
        <span>{`LN ${formatTwoDigits(cursor.line)}`}</span>
        <span>{`COL ${formatTwoDigits(cursor.col)}`}</span>
        <span>{`MODE: ${chrome.modeLabel}`}</span>
        <span>{chrome.insertLabel}</span>
        <span>{chrome.wrapLabel}</span>
        <span>{chrome.memoryLabel}</span>
        <span>{`[SYSTIME: ${formatDateTimeStamp(now)}]`}</span>
      </div>

      <div className="alien-nostromo__editor-shell">
        <div className="alien-nostromo__editor">{editorPane}</div>
      </div>

      <div className="alien-nostromo__command-strip">
        {chrome.commandHints.map((hint) => (
          <span key={hint}>{hint}</span>
        ))}
      </div>

      <div className="alien-nostromo__footer">{chrome.footerHint}</div>
    </div>
  );
}

function C64SpeedwriteThemeRenderer({
  cursor,
  editorPane,
  theme,
}: ThemeRendererProps) {
  const chrome = getThemeChrome(theme, 'c64-speedwrite');

  if (!chrome) {
    return null;
  }

  return (
    <div className="c64-screen c64-screen--speedwrite">
      <div className="c64-speedwrite__header">
        <span>{chrome.title}</span>
        <span>{`FILE: ${chrome.documentName}`}</span>
        <span>{`L${formatTwoDigits(cursor.line)}`}</span>
        <span>{`C${formatTwoDigits(cursor.col)}`}</span>
        <span>{`MODE: ${chrome.modeLabel}`}</span>
      </div>

      <div className="c64-speedwrite__editor">{editorPane}</div>

      <div className="c64-speedwrite__footer">
        {chrome.footerHints.map((hint) => (
          <span key={hint}>{hint}</span>
        ))}
      </div>
    </div>
  );
}

function C64EasyScriptThemeRenderer({
  cursor,
  editorPane,
  theme,
}: ThemeRendererProps) {
  const chrome = getThemeChrome(theme, 'c64-easycript');

  if (!chrome) {
    return null;
  }

  return (
    <div className="c64-screen c64-screen--easycript">
      <div className="c64-easycript__title">{chrome.title}</div>

      <div className="c64-easycript__status">
        <span>{`FILE: ${chrome.documentName}`}</span>
        <span>{`LINE: ${cursor.line}`}</span>
        <span>{`COL: ${cursor.col}`}</span>
        <span>{chrome.modeLabel}</span>
      </div>

      <div className="c64-easycript__editor-shell">
        <div className="c64-easycript__editor">{editorPane}</div>
      </div>
    </div>
  );
}

function C64WriterThemeRenderer({
  cursor,
  editorPane,
  theme,
}: ThemeRendererProps) {
  const chrome = getThemeChrome(theme, 'c64-writer');

  if (!chrome) {
    return null;
  }

  return (
    <div className="c64-screen c64-screen--writer">
      <div className="c64-writer__header">
        <span>{chrome.title}</span>
        <span>{`COL: ${cursor.col}`}</span>
        <span>{`ROW: ${cursor.line}`}</span>
        <span>{`FILE: ${chrome.documentName}`}</span>
      </div>

      <div className="c64-writer__editor-shell">
        <div className="c64-writer__marker" aria-hidden="true" />
        <div className="c64-writer__editor">{editorPane}</div>
      </div>

      <div className="c64-writer__footer">{chrome.bottomBarLabel}</div>
    </div>
  );
}

function C64FunctionBarThemeRenderer({
  cursor,
  editorPane,
  theme,
}: ThemeRendererProps) {
  const chrome = getThemeChrome(theme, 'c64-function-bar');

  if (!chrome) {
    return null;
  }

  return (
    <div className="c64-screen c64-screen--function-bar">
      <div className="c64-function-bar__commands">
        {chrome.commandHints.map((hint) => (
          <span key={hint}>{hint}</span>
        ))}
      </div>

      <div className="c64-function-bar__status">
        <span>{`FILE: ${chrome.documentName}`}</span>
        <span>{`LN: ${formatTwoDigits(cursor.line)}`}</span>
        <span>{`COL: ${formatTwoDigits(cursor.col)}`}</span>
        <span>{`CAPS: ${chrome.capsLabel}`}</span>
        <span>{`MODIFIED: ${chrome.modifiedLabel}`}</span>
      </div>

      <div className="c64-function-bar__separator" aria-hidden="true">
        {chrome.separatorText}
      </div>

      <div className="c64-function-bar__editor-shell">
        <div className="c64-function-bar__editor">{editorPane}</div>
      </div>
    </div>
  );
}

const THEME_RENDERERS = {
  classic: ClassicThemeRenderer,
  'status-bar': StatusBarThemeRenderer,
  ruler: RulerThemeRenderer,
  bracketed: BracketedThemeRenderer,
  writing: WritingThemeRenderer,
  'appleworks-menu': AppleworksMenuThemeRenderer,
  'applewriter-family': ApplewriterFamilyThemeRenderer,
  'banner-footer': BannerFooterThemeRenderer,
  'framed-terminal': FramedTerminalThemeRenderer,
  'unix-status': UnixStatusThemeRenderer,
  'starfleet-command': StarfleetCommandThemeRenderer,
  'enterprise-systems': EnterpriseSystemsThemeRenderer,
  'lcars-4-1': Lcars41ThemeRenderer,
  'mother-6800': Mother6800ThemeRenderer,
  'ready-input': ReadyInputThemeRenderer,
  'nostromo-data-systems': NostromoDataSystemsThemeRenderer,
  'c64-speedwrite': C64SpeedwriteThemeRenderer,
  'c64-easycript': C64EasyScriptThemeRenderer,
  'c64-writer': C64WriterThemeRenderer,
  'c64-function-bar': C64FunctionBarThemeRenderer,
} satisfies Record<ThemeVariant, ThemeRenderer>;

export function renderThemeContent(props: ThemeRendererProps) {
  const Renderer = THEME_RENDERERS[props.theme.chrome.variant];
  return <Renderer {...props} />;
}
