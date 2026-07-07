export type ThemeId =
  | 'classic'
  | 'status-bar-green'
  | 'amber-ruler'
  | 'bracketed-green'
  | 'amber-writing'
  | 'starfleet-command'
  | 'enterprise-systems'
  | 'lcars-4-1'
  | 'mother-6800'
  | 'ready-input'
  | 'nostromo-data-systems'
  | 'c64-speedwrite'
  | 'c64-easycript'
  | 'c64-writer'
  | 'c64-function-bar'
  | 'appleworks-1-1'
  | 'apple-writer-ii'
  | 'apple-ii-word-processor'
  | 'applewriter-ii-v1-1'
  | 'amber-editor'
  | 'nwriter-amber'
  | 'uw-unix-nano'
  | 'unix-vi-emacs'
  | 'pip-os-word-processor'
  | 'robco-unified-os'
  | 'vault-tec-data-terminal'
  | 'robco-script-o-matic';

export type ThemeVariant =
  | 'classic'
  | 'status-bar'
  | 'ruler'
  | 'bracketed'
  | 'writing'
  | 'appleworks-menu'
  | 'applewriter-family'
  | 'banner-footer'
  | 'framed-terminal'
  | 'unix-status'
  | 'starfleet-command'
  | 'enterprise-systems'
  | 'lcars-4-1'
  | 'mother-6800'
  | 'ready-input'
  | 'nostromo-data-systems'
  | 'c64-speedwrite'
  | 'c64-easycript'
  | 'c64-writer'
  | 'c64-function-bar';

export type LcarsTone =
  | 'peach'
  | 'gold'
  | 'yellow'
  | 'orange'
  | 'red'
  | 'blue'
  | 'sky'
  | 'violet'
  | 'pink'
  | 'mint'
  | 'cream';

export interface ThemeDefinition {
  id: ThemeId;
  label: string;
  screenLayout?: {
    scale?: string;
    safeInset?: string;
  };
  screenFx: {
    appBackground: string;
    roomGlow: string;
    roomGridMajor: string;
    roomGridMinor: string;
    roomGridOpacity: string;
    frameShadow: string;
    monitorUnderlay: string;
    screenInset: string;
    screenBackground: string;
    screenTint: string;
    screenVignette: string;
    screenInnerShadow: string;
    scanline: string;
    scanlineOpacity: string;
    noiseOpacity: string;
    textGlow: string;
    contentPaddingTop: string;
    contentPaddingRight: string;
    contentPaddingBottom: string;
    contentPaddingLeft: string;
  };
  chrome: ThemeChrome;
  editorLayout: {
    fontFamily: string;
    fontSize: string;
    lineHeight: string;
    letterSpacing: string;
    fontWeight: string;
    placeholder: string;
    caretColor: string;
    selectionColor: string;
    prompt?: string;
    promptColor?: string;
    paddingTop: string;
    paddingRight: string;
    paddingBottom: string;
    paddingLeft: string;
    gutter?: {
      lineCount: number;
      width: string;
      gap: string;
      color: string;
      activeColor: string;
      fontSize: string;
      formatter?: (lineNumber: number) => string;
    };
  };
  legend?: {
    lines: string[][];
  };
}

interface ThemeChromeBase {
  variant: ThemeVariant;
  textColor: string;
  accentColor: string;
  dimTextColor: string;
  dividerColor: string;
  inverseTextColor?: string;
  inverseBackground?: string;
}

export interface ClassicThemeChrome extends ThemeChromeBase {
  variant: 'classic';
  fileName: string;
  pathText: string;
  documentTitle: string;
  modeLabel: string;
  menuItems: string[];
}

export interface StatusBarThemeChrome extends ThemeChromeBase {
  variant: 'status-bar';
  fileName: string;
  modeLabel: string;
  directoryLabel: string;
  menuLabel: string;
}

export interface RulerThemeChrome extends ThemeChromeBase {
  variant: 'ruler';
  fileName: string;
  modeLabel: string;
  rulerText: string;
}

export interface BracketedThemeChrome extends ThemeChromeBase {
  variant: 'bracketed';
  fileName: string;
  modeLabel: string;
  insertLabel: string;
}

export interface WritingThemeChrome extends ThemeChromeBase {
  variant: 'writing';
  fileName: string;
  modeLabel: string;
  pageLabel: string;
  commandField: string;
}

type ThemeTextAlign = 'left' | 'center' | 'right';

interface ThemeTextLine {
  text: string;
  align?: ThemeTextAlign;
  inverse?: boolean;
}

export interface AppleworksMenuThemeChrome extends ThemeChromeBase {
  variant: 'appleworks-menu';
  mastheadLeft: string;
  mastheadCenter: string;
  mastheadRight: string;
  menuItems: string[];
  fileLabel: string;
  pageLabel: string;
  lineLabel: string;
}

export interface ApplewriterFamilyThemeChrome extends ThemeChromeBase {
  variant: 'applewriter-family';
  titleLine: string;
  titleAlign?: ThemeTextAlign;
  statusLine?: string;
  statusAlign?: ThemeTextAlign;
}

export interface BannerFooterThemeChrome extends ThemeChromeBase {
  variant: 'banner-footer';
  headerLines: ThemeTextLine[];
  footerLines: ThemeTextLine[];
  dividerAfterHeader?: boolean;
  dividerBeforeFooter?: boolean;
  editorBordered?: boolean;
}

export interface FramedTerminalThemeChrome extends ThemeChromeBase {
  variant: 'framed-terminal';
  headerLeft: string;
  headerCenter: string;
  headerRight: string;
  statusLabel: string;
  commandHints: string[];
}

export interface UnixStatusThemeChrome extends ThemeChromeBase {
  variant: 'unix-status';
  titleLine: string;
  metaLine: string;
  footerLeft: string;
  footerRight: string;
}

export interface LcarsToneDefinition {
  background: string;
  textColor?: string;
}

export interface LcarsAction {
  label: string;
  tone: LcarsTone;
}

export interface LcarsRailSegment {
  tone: LcarsTone;
  flex?: number;
  label?: string;
}

export interface StarfleetCommandThemeChrome extends ThemeChromeBase {
  variant: 'starfleet-command';
  tones: Partial<Record<LcarsTone, LcarsToneDefinition>>;
  mastheadLines: [string, string];
  title: string;
  topActions: LcarsAction[];
  upperRails: LcarsRailSegment[];
  lowerRails: LcarsRailSegment[];
  leftActions: LcarsAction[];
  rightPanels: LcarsAction[];
  documentName: string;
  statusText: string;
  editorHeading: string;
  footerFields: string[];
}

export interface EnterpriseSystemsThemeChrome extends ThemeChromeBase {
  variant: 'enterprise-systems';
  tones: Partial<Record<LcarsTone, LcarsToneDefinition>>;
  headerLeft: string;
  headerRight: string;
  subtitle: string;
  upperRails: LcarsRailSegment[];
  lowerRails: LcarsRailSegment[];
  primaryActions: LcarsAction[];
  secondaryActions: LcarsAction[];
  documentName: string;
  stardate: string;
  footerMenu: string[];
  loggedInAs: string;
  terminal: string;
  securityText: string;
  timeZone: string;
}

export interface Lcars41ThemeChrome extends ThemeChromeBase {
  variant: 'lcars-4-1';
  tones: Partial<Record<LcarsTone, LcarsToneDefinition>>;
  title: string;
  commandLine: string;
  metaFields: string[];
  systemStatus: string;
  networkStatus: string;
  editorTitle: string;
  leftActions: LcarsAction[];
  rightActions: LcarsAction[];
  bottomActions: LcarsAction[];
  fileFooter: string;
}

export interface Mother6800ThemeChrome extends ThemeChromeBase {
  variant: 'mother-6800';
  systemName: string;
  consoleName: string;
  documentName: string;
  pageLabel: string;
  modeLabel: string;
  menuItems: string[];
  railItems: string[];
  promptLabel: string;
  footerLeft: string;
  footerRight: string;
}

export interface ReadyInputThemeChrome extends ThemeChromeBase {
  variant: 'ready-input';
  documentName: string;
  statusLabel: string;
  pageLabel: string;
  commandHints: string[];
  footerHint: string;
}

export interface NostromoDataSystemsThemeChrome extends ThemeChromeBase {
  variant: 'nostromo-data-systems';
  headerTitle: string;
  accessLabel: string;
  editorStatus: string;
  documentName: string;
  modeLabel: string;
  insertLabel: string;
  wrapLabel: string;
  memoryLabel: string;
  commandHints: string[];
  footerHint: string;
}

export interface C64SpeedwriteThemeChrome extends ThemeChromeBase {
  variant: 'c64-speedwrite';
  title: string;
  documentName: string;
  modeLabel: string;
  footerHints: string[];
}

export interface C64EasyScriptThemeChrome extends ThemeChromeBase {
  variant: 'c64-easycript';
  title: string;
  documentName: string;
  modeLabel: string;
}

export interface C64WriterThemeChrome extends ThemeChromeBase {
  variant: 'c64-writer';
  title: string;
  documentName: string;
  bottomBarLabel: string;
}

export interface C64FunctionBarThemeChrome extends ThemeChromeBase {
  variant: 'c64-function-bar';
  commandHints: string[];
  documentName: string;
  capsLabel: string;
  modifiedLabel: string;
  separatorText: string;
}

export type ThemeChrome =
  | ClassicThemeChrome
  | StatusBarThemeChrome
  | RulerThemeChrome
  | BracketedThemeChrome
  | WritingThemeChrome
  | AppleworksMenuThemeChrome
  | ApplewriterFamilyThemeChrome
  | BannerFooterThemeChrome
  | FramedTerminalThemeChrome
  | UnixStatusThemeChrome
  | StarfleetCommandThemeChrome
  | EnterpriseSystemsThemeChrome
  | Lcars41ThemeChrome
  | Mother6800ThemeChrome
  | ReadyInputThemeChrome
  | NostromoDataSystemsThemeChrome
  | C64SpeedwriteThemeChrome
  | C64EasyScriptThemeChrome
  | C64WriterThemeChrome
  | C64FunctionBarThemeChrome;

export const DEFAULT_THEME_ID: ThemeId = 'classic';

export const THEME_ORDER: ThemeId[] = [
  'classic',
  'status-bar-green',
  'amber-ruler',
  'bracketed-green',
  'amber-writing',
  'starfleet-command',
  'enterprise-systems',
  'lcars-4-1',
  'mother-6800',
  'ready-input',
  'nostromo-data-systems',
  'c64-speedwrite',
  'c64-easycript',
  'c64-writer',
  'c64-function-bar',
  'appleworks-1-1',
  'apple-writer-ii',
  'apple-ii-word-processor',
  'applewriter-ii-v1-1',
  'amber-editor',
  'nwriter-amber',
  'uw-unix-nano',
  'unix-vi-emacs',
  'pip-os-word-processor',
  'robco-unified-os',
  'vault-tec-data-terminal',
  'robco-script-o-matic',
];

const lcarsFontFamily =
  '"Arial Narrow", "Aptos Narrow", "Liberation Sans Narrow", "Bahnschrift", sans-serif';

const alienFontFamily =
  '"Cascadia Mono", "Aptos Mono", "Consolas", "Lucida Console", monospace';

const c64FontFamily =
  '"Cascadia Mono", "Lucida Console", "Courier New", monospace';

const lcarsEditorLayout = {
  fontFamily: lcarsFontFamily,
  fontSize: 'clamp(0.96rem, 1.24vw, 1.44rem)',
  lineHeight: '1.36',
  letterSpacing: '0.02em',
  fontWeight: '700',
  placeholder: '',
  caretColor: '#ffd261',
  selectionColor: 'rgba(255, 194, 92, 0.18)',
  paddingTop: '0.1rem',
  paddingRight: '0.1rem',
  paddingBottom: '0',
  paddingLeft: '0.1rem',
} as const;

const alienEditorLayout = {
  fontFamily: alienFontFamily,
  fontSize: 'clamp(0.96rem, 1.22vw, 1.38rem)',
  lineHeight: '1.42',
  letterSpacing: '0.038em',
  fontWeight: '400',
  placeholder: '',
  caretColor: '#abff9f',
  selectionColor: 'rgba(157, 255, 147, 0.16)',
  paddingTop: '0',
  paddingRight: '0',
  paddingBottom: '0',
  paddingLeft: '0',
} as const;

const c64EditorLayout = {
  fontFamily: c64FontFamily,
  fontSize: 'clamp(0.98rem, 1.2vw, 1.38rem)',
  lineHeight: '1.34',
  letterSpacing: '0.05em',
  fontWeight: '700',
  placeholder: '',
  caretColor: '#a9d8ff',
  selectionColor: 'rgba(188, 223, 255, 0.2)',
  paddingTop: '0',
  paddingRight: '0',
  paddingBottom: '0',
  paddingLeft: '0',
} as const;

const c64ScreenFxBase = {
  appBackground: '#06091b',
  roomGlow:
    'radial-gradient(circle at center, rgba(95, 124, 255, 0.28), rgba(15, 20, 54, 0.94) 56%, rgba(5, 7, 18, 1) 100%)',
  roomGridMajor: 'rgba(171, 201, 255, 0.026)',
  roomGridMinor: 'rgba(171, 201, 255, 0.014)',
  roomGridOpacity: '0.05',
  frameShadow:
    '0 0 0 1px rgba(154, 195, 255, 0.08), 0 0 88px rgba(0, 0, 0, 0.62)',
  monitorUnderlay: '#080b22',
  screenInset: '8.5% 6.6% 11.2% 6.6%',
  screenBackground: '#27357d',
  screenTint:
    'radial-gradient(circle at 50% 18%, rgba(169, 205, 255, 0.08), rgba(38, 53, 124, 0.9) 58%, rgba(18, 28, 79, 0.98) 100%)',
  screenVignette:
    'radial-gradient(circle at 50% 48%, rgba(0, 0, 0, 0) 58%, rgba(0, 0, 0, 0.34) 100%)',
  screenInnerShadow:
    'inset 0 0 38px rgba(0, 0, 0, 0.46), inset 0 0 16px rgba(176, 210, 255, 0.08)',
  scanline:
    'repeating-linear-gradient(to bottom, rgba(193, 223, 255, 0.032) 0, rgba(193, 223, 255, 0.032) 1px, transparent 2px, transparent 4px)',
  scanlineOpacity: '0.18',
  noiseOpacity: '0.06',
  textGlow: '0 0 8px rgba(183, 215, 255, 0.12)',
  contentPaddingTop: '0.78rem',
  contentPaddingRight: '0.88rem',
  contentPaddingBottom: '0.82rem',
  contentPaddingLeft: '0.88rem',
} as const;

const appleScreenFxBase = {
  appBackground: '#040603',
  roomGlow:
    'radial-gradient(circle at center, rgba(44, 96, 52, 0.34), rgba(7, 13, 8, 0.95) 58%, rgba(1, 3, 1, 1) 100%)',
  roomGridMajor: 'rgba(138, 255, 156, 0.03)',
  roomGridMinor: 'rgba(138, 255, 156, 0.016)',
  roomGridOpacity: '0.06',
  frameShadow:
    '0 0 0 1px rgba(124, 255, 149, 0.08), 0 0 84px rgba(0, 0, 0, 0.62)',
  monitorUnderlay: '#030502',
  screenInset: '8.5% 6.6% 11.2% 6.6%',
  screenBackground: '#021204',
  screenTint:
    'radial-gradient(circle at 50% 24%, rgba(109, 255, 136, 0.12), rgba(6, 21, 9, 0.92) 60%, rgba(2, 8, 3, 0.98) 100%)',
  screenVignette:
    'radial-gradient(circle at 50% 50%, rgba(0, 0, 0, 0) 58%, rgba(0, 0, 0, 0.46) 100%)',
  screenInnerShadow:
    'inset 0 0 46px rgba(0, 0, 0, 0.68), inset 0 0 18px rgba(86, 188, 102, 0.08)',
  scanline:
    'repeating-linear-gradient(to bottom, rgba(137, 255, 157, 0.038) 0, rgba(137, 255, 157, 0.038) 1px, transparent 2px, transparent 4px)',
  scanlineOpacity: '0.34',
  noiseOpacity: '0.11',
  textGlow: '0 0 9px rgba(132, 255, 148, 0.16)',
  contentPaddingTop: '0.92rem',
  contentPaddingRight: '0.96rem',
  contentPaddingBottom: '0.96rem',
  contentPaddingLeft: '0.96rem',
} as const;

const amberScreenFxBase = {
  appBackground: '#0a0502',
  roomGlow:
    'radial-gradient(circle at center, rgba(110, 60, 14, 0.32), rgba(15, 8, 4, 0.95) 60%, rgba(4, 2, 1, 1) 100%)',
  roomGridMajor: 'rgba(255, 189, 105, 0.024)',
  roomGridMinor: 'rgba(255, 189, 105, 0.012)',
  roomGridOpacity: '0.05',
  frameShadow:
    '0 0 0 1px rgba(255, 187, 112, 0.07), 0 0 88px rgba(0, 0, 0, 0.66)',
  monitorUnderlay: '#090402',
  screenInset: '8.5% 6.6% 11.2% 6.6%',
  screenBackground: '#140803',
  screenTint:
    'radial-gradient(circle at 50% 24%, rgba(255, 168, 70, 0.12), rgba(78, 31, 8, 0.24) 44%, rgba(20, 8, 3, 0.98) 100%)',
  screenVignette:
    'radial-gradient(circle at 50% 50%, rgba(0, 0, 0, 0) 58%, rgba(0, 0, 0, 0.44) 100%)',
  screenInnerShadow:
    'inset 0 0 46px rgba(0, 0, 0, 0.68), inset 0 0 20px rgba(198, 113, 32, 0.1)',
  scanline:
    'repeating-linear-gradient(to bottom, rgba(255, 183, 95, 0.032) 0, rgba(255, 183, 95, 0.032) 1px, transparent 2px, transparent 5px)',
  scanlineOpacity: '0.3',
  noiseOpacity: '0.12',
  textGlow: '0 0 8px rgba(255, 188, 107, 0.14)',
  contentPaddingTop: '0.96rem',
  contentPaddingRight: '1.02rem',
  contentPaddingBottom: '0.98rem',
  contentPaddingLeft: '1.02rem',
} as const;

const robcoGreenScreenFxBase = {
  appBackground: '#050805',
  roomGlow:
    'radial-gradient(circle at center, rgba(42, 92, 56, 0.34), rgba(9, 17, 11, 0.94) 58%, rgba(3, 5, 4, 1) 100%)',
  roomGridMajor: 'rgba(138, 255, 167, 0.024)',
  roomGridMinor: 'rgba(138, 255, 167, 0.012)',
  roomGridOpacity: '0.05',
  frameShadow:
    '0 0 0 1px rgba(129, 232, 160, 0.07), 0 0 88px rgba(0, 0, 0, 0.66)',
  monitorUnderlay: '#060906',
  screenInset: '8.5% 6.6% 11.2% 6.6%',
  screenBackground: '#07160b',
  screenTint:
    'radial-gradient(circle at 50% 26%, rgba(98, 192, 125, 0.14), rgba(10, 29, 16, 0.9) 54%, rgba(4, 10, 6, 0.98) 100%)',
  screenVignette:
    'radial-gradient(circle at 50% 50%, rgba(0, 0, 0, 0) 58%, rgba(0, 0, 0, 0.44) 100%)',
  screenInnerShadow:
    'inset 0 0 46px rgba(0, 0, 0, 0.68), inset 0 0 18px rgba(91, 184, 116, 0.08)',
  scanline:
    'repeating-linear-gradient(to bottom, rgba(147, 255, 177, 0.03) 0, rgba(147, 255, 177, 0.03) 1px, transparent 2px, transparent 5px)',
  scanlineOpacity: '0.26',
  noiseOpacity: '0.1',
  textGlow:
    '0 0 8px rgba(255, 185, 103, 0.12), 0 0 11px rgba(133, 255, 167, 0.08)',
  contentPaddingTop: '0.9rem',
  contentPaddingRight: '0.98rem',
  contentPaddingBottom: '0.96rem',
  contentPaddingLeft: '0.98rem',
} as const;

export const THEMES: ThemeDefinition[] = [
  {
    id: 'classic',
    label: 'Classic CRT',
    screenLayout: {
      scale: '1',
      safeInset: '0',
    },
    screenFx: {
      appBackground: '#241d17',
      roomGlow:
        'radial-gradient(circle at center, rgba(120, 108, 90, 0.5), rgba(62, 53, 43, 0.9) 55%, rgba(25, 21, 17, 1) 100%)',
      roomGridMajor: 'rgba(255, 255, 255, 0.06)',
      roomGridMinor: 'rgba(255, 255, 255, 0.03)',
      roomGridOpacity: '0.12',
      frameShadow:
        '0 0 0 1px rgba(121, 255, 189, 0.08), 0 0 70px rgba(0, 0, 0, 0.45)',
      monitorUnderlay: '#050805',
      screenInset: '8.5% 6.6% 11.2% 6.6%',
      screenBackground: '#050905',
      screenTint:
        'radial-gradient(circle at 50% 35%, rgba(89, 173, 127, 0.18), rgba(8, 15, 11, 0.98) 74%)',
      screenVignette:
        'radial-gradient(circle at 50% 50%, rgba(0, 0, 0, 0) 54%, rgba(0, 0, 0, 0.48) 100%)',
      screenInnerShadow: 'inset 0 0 45px rgba(0, 0, 0, 0.8)',
      scanline:
        'repeating-linear-gradient(to bottom, rgba(148, 255, 209, 0.024) 0, rgba(148, 255, 209, 0.024) 1px, transparent 3px, transparent 6px)',
      scanlineOpacity: '0.7',
      noiseOpacity: '0.1',
      textGlow: '0 0 10px rgba(141, 241, 193, 0.2)',
      contentPaddingTop: '6.2%',
      contentPaddingRight: '6.2%',
      contentPaddingBottom: '5.5%',
      contentPaddingLeft: '6.2%',
    },
    chrome: {
      variant: 'classic',
      textColor: '#7ee9b9',
      accentColor: '#90f8c8',
      dimTextColor: '#70b893',
      dividerColor: '#73e3b0',
      fileName: 'untitled.txt',
      pathText: 'C:\\DOCS> untitled.txt',
      documentTitle: 'Sample Document',
      modeLabel: 'INSERT',
      menuItems: ['File', 'Edit', 'View', 'Tools'],
    },
    editorLayout: {
      fontFamily: 'Georgia, "Times New Roman", serif',
      fontSize: 'clamp(1.1rem, 1.7vw, 2.15rem)',
      lineHeight: '1.34',
      letterSpacing: '0.005em',
      fontWeight: '400',
      placeholder: 'Start typing...',
      caretColor: '#96fbcf',
      selectionColor: 'rgba(135, 255, 197, 0.28)',
      paddingTop: '0',
      paddingRight: '0',
      paddingBottom: '0',
      paddingLeft: '0',
    },
  },
  {
    id: 'status-bar-green',
    label: 'Status Bar Green',
    screenLayout: {
      scale: '0.93',
      safeInset: '2.2% 1.8% 2.6%',
    },
    screenFx: {
      appBackground: '#060a04',
      roomGlow:
        'radial-gradient(circle at center, rgba(24, 63, 28, 0.42), rgba(4, 10, 5, 0.94) 60%, rgba(2, 4, 2, 1) 100%)',
      roomGridMajor: 'rgba(120, 255, 140, 0.035)',
      roomGridMinor: 'rgba(120, 255, 140, 0.02)',
      roomGridOpacity: '0.08',
      frameShadow:
        '0 0 0 1px rgba(100, 255, 124, 0.08), 0 0 80px rgba(0, 0, 0, 0.58)',
      monitorUnderlay: '#020603',
      screenInset: '8.5% 6.6% 11.2% 6.6%',
      screenBackground: '#031706',
      screenTint:
        'radial-gradient(circle at 50% 28%, rgba(96, 245, 111, 0.16), rgba(8, 24, 10, 0.92) 68%, rgba(1, 7, 2, 0.98) 100%)',
      screenVignette:
        'radial-gradient(circle at 50% 48%, rgba(0, 0, 0, 0) 56%, rgba(0, 0, 0, 0.4) 100%)',
      screenInnerShadow:
        'inset 0 0 42px rgba(0, 0, 0, 0.62), inset 0 0 18px rgba(30, 112, 42, 0.14)',
      scanline:
        'repeating-linear-gradient(to bottom, rgba(135, 255, 154, 0.05) 0, rgba(135, 255, 154, 0.05) 1px, transparent 2px, transparent 4px)',
      scanlineOpacity: '0.38',
      noiseOpacity: '0.14',
      textGlow: '0 0 9px rgba(132, 255, 144, 0.16)',
      contentPaddingTop: '0',
      contentPaddingRight: '0',
      contentPaddingBottom: '0',
      contentPaddingLeft: '0',
    },
    chrome: {
      variant: 'status-bar',
      textColor: '#7bf588',
      accentColor: '#92ff98',
      dimTextColor: '#62cf70',
      dividerColor: '#78ef82',
      inverseTextColor: '#123017',
      inverseBackground: '#91f495',
      fileName: '[Untitled]',
      modeLabel: 'Ins',
      directoryLabel: '<Dir>',
      menuLabel: '[Menu]',
    },
    editorLayout: {
      fontFamily: '"Lucida Console", "Courier New", monospace',
      fontSize: 'clamp(1rem, 1.52vw, 1.9rem)',
      lineHeight: '1.34',
      letterSpacing: '0.03em',
      fontWeight: '400',
      placeholder: '',
      caretColor: '#9dffa7',
      selectionColor: 'rgba(131, 255, 137, 0.22)',
      paddingTop: '1rem',
      paddingRight: '1rem',
      paddingBottom: '1rem',
      paddingLeft: '0.15rem',
      gutter: {
        lineCount: 24,
        width: '3.25rem',
        gap: '0.55rem',
        color: '#60b968',
        activeColor: '#9df8a3',
        fontSize: 'clamp(0.92rem, 1.28vw, 1.48rem)',
      },
    },
  },
  {
    id: 'amber-ruler',
    label: 'Amber Ruler',
    screenLayout: {
      scale: '0.94',
      safeInset: '1.2% 1.6% 2.8%',
    },
    screenFx: {
      appBackground: '#0c0704',
      roomGlow:
        'radial-gradient(circle at center, rgba(98, 49, 10, 0.28), rgba(15, 8, 4, 0.94) 62%, rgba(4, 2, 1, 1) 100%)',
      roomGridMajor: 'rgba(255, 191, 121, 0.03)',
      roomGridMinor: 'rgba(255, 191, 121, 0.015)',
      roomGridOpacity: '0.05',
      frameShadow:
        '0 0 0 1px rgba(255, 187, 112, 0.06), 0 0 80px rgba(0, 0, 0, 0.62)',
      monitorUnderlay: '#090503',
      screenInset: '8.5% 6.6% 11.2% 6.6%',
      screenBackground: '#0f1204',
      screenTint:
        'radial-gradient(circle at 50% 36%, rgba(177, 116, 35, 0.16), rgba(37, 37, 7, 0.5) 48%, rgba(9, 16, 5, 0.95) 100%)',
      screenVignette:
        'radial-gradient(circle at 50% 44%, rgba(0, 0, 0, 0) 52%, rgba(0, 0, 0, 0.52) 100%)',
      screenInnerShadow:
        'inset 0 0 44px rgba(0, 0, 0, 0.64), inset 0 0 24px rgba(162, 107, 42, 0.1)',
      scanline:
        'repeating-linear-gradient(to bottom, rgba(255, 186, 109, 0.032) 0, rgba(255, 186, 109, 0.032) 1px, transparent 2px, transparent 5px)',
      scanlineOpacity: '0.34',
      noiseOpacity: '0.12',
      textGlow: '0 0 9px rgba(255, 186, 109, 0.14)',
      contentPaddingTop: '1.35rem',
      contentPaddingRight: '1.7rem',
      contentPaddingBottom: '1.55rem',
      contentPaddingLeft: '1.8rem',
    },
    chrome: {
      variant: 'ruler',
      textColor: '#f7af63',
      accentColor: '#ffc27a',
      dimTextColor: '#d68f48',
      dividerColor: 'rgba(255, 195, 127, 0.86)',
      fileName: 'NEWFILE.TXT',
      modeLabel: 'EDIT',
      rulerText:
        '.....!....1....!....2....!....3....!....4....!....5....!....6....!....7...V',
    },
    editorLayout: {
      fontFamily: '"Lucida Console", "Courier New", monospace',
      fontSize: 'clamp(1rem, 1.42vw, 1.76rem)',
      lineHeight: '1.42',
      letterSpacing: '0.035em',
      fontWeight: '400',
      placeholder: '',
      caretColor: '#ffc675',
      selectionColor: 'rgba(255, 194, 116, 0.18)',
      paddingTop: '1rem',
      paddingRight: '0',
      paddingBottom: '0.5rem',
      paddingLeft: '0',
    },
    legend: {
      lines: [
        ['^G Help', '^O Open', '^W Write', '^R Read', '^P Print', '^C Center'],
        ['^X Exit', '^Y Lock', '^T ReCon', '^K Center', '^L Lock', '^S Search'],
      ],
    },
  },
  {
    id: 'bracketed-green',
    label: 'Bracketed Green',
    screenLayout: {
      scale: '0.95',
      safeInset: '1.2% 1.6% 2.2%',
    },
    screenFx: {
      appBackground: '#050805',
      roomGlow:
        'radial-gradient(circle at center, rgba(16, 58, 27, 0.34), rgba(7, 12, 7, 0.95) 58%, rgba(1, 3, 1, 1) 100%)',
      roomGridMajor: 'rgba(120, 255, 149, 0.03)',
      roomGridMinor: 'rgba(120, 255, 149, 0.02)',
      roomGridOpacity: '0.06',
      frameShadow:
        '0 0 0 1px rgba(119, 255, 154, 0.07), 0 0 78px rgba(0, 0, 0, 0.6)',
      monitorUnderlay: '#020402',
      screenInset: '8.5% 6.6% 11.2% 6.6%',
      screenBackground: '#031206',
      screenTint:
        'radial-gradient(circle at 50% 30%, rgba(60, 240, 103, 0.16), rgba(7, 27, 11, 0.94) 66%, rgba(2, 10, 3, 0.98) 100%)',
      screenVignette:
        'radial-gradient(circle at 50% 52%, rgba(0, 0, 0, 0) 60%, rgba(0, 0, 0, 0.46) 100%)',
      screenInnerShadow:
        'inset 0 0 44px rgba(0, 0, 0, 0.66), inset 0 0 18px rgba(52, 175, 83, 0.08)',
      scanline:
        'repeating-linear-gradient(to bottom, rgba(114, 255, 149, 0.035) 0, rgba(114, 255, 149, 0.035) 1px, transparent 2px, transparent 4px)',
      scanlineOpacity: '0.32',
      noiseOpacity: '0.12',
      textGlow: '0 0 10px rgba(108, 255, 142, 0.17)',
      contentPaddingTop: '1.5rem',
      contentPaddingRight: '1.85rem',
      contentPaddingBottom: '1.15rem',
      contentPaddingLeft: '1.9rem',
    },
    chrome: {
      variant: 'bracketed',
      textColor: '#74ff8f',
      accentColor: '#9cffaf',
      dimTextColor: '#63cf78',
      dividerColor: 'rgba(134, 255, 157, 0.88)',
      fileName: 'UNTITLED.TXT',
      modeLabel: 'EDIT',
      insertLabel: 'INS',
    },
    editorLayout: {
      fontFamily: '"Lucida Console", "Courier New", monospace',
      fontSize: 'clamp(1.05rem, 1.62vw, 2rem)',
      lineHeight: '1.42',
      letterSpacing: '0.03em',
      fontWeight: '400',
      placeholder: '',
      caretColor: '#9dffab',
      selectionColor: 'rgba(133, 255, 151, 0.2)',
      prompt: '>',
      promptColor: '#8efe98',
      paddingTop: '1rem',
      paddingRight: '0',
      paddingBottom: '0',
      paddingLeft: '1.8rem',
    },
  },
  {
    id: 'amber-writing',
    label: 'Amber Writing',
    screenLayout: {
      scale: '0.94',
      safeInset: '1.2% 1.8% 2.8%',
    },
    screenFx: {
      appBackground: '#0c0603',
      roomGlow:
        'radial-gradient(circle at center, rgba(92, 50, 13, 0.28), rgba(12, 7, 4, 0.95) 60%, rgba(4, 2, 1, 1) 100%)',
      roomGridMajor: 'rgba(255, 181, 91, 0.025)',
      roomGridMinor: 'rgba(255, 181, 91, 0.012)',
      roomGridOpacity: '0.04',
      frameShadow:
        '0 0 0 1px rgba(255, 178, 83, 0.06), 0 0 82px rgba(0, 0, 0, 0.64)',
      monitorUnderlay: '#080402',
      screenInset: '8.5% 6.6% 11.2% 6.6%',
      screenBackground: '#180904',
      screenTint:
        'radial-gradient(circle at 50% 24%, rgba(235, 140, 42, 0.12), rgba(84, 32, 9, 0.26) 44%, rgba(24, 9, 4, 0.98) 100%)',
      screenVignette:
        'radial-gradient(circle at 50% 48%, rgba(0, 0, 0, 0) 56%, rgba(0, 0, 0, 0.42) 100%)',
      screenInnerShadow:
        'inset 0 0 44px rgba(0, 0, 0, 0.62), inset 0 0 20px rgba(201, 110, 30, 0.12)',
      scanline:
        'repeating-linear-gradient(to bottom, rgba(255, 179, 85, 0.03) 0, rgba(255, 179, 85, 0.03) 1px, transparent 2px, transparent 5px)',
      scanlineOpacity: '0.3',
      noiseOpacity: '0.12',
      textGlow: '0 0 9px rgba(255, 181, 91, 0.14)',
      contentPaddingTop: '1.5rem',
      contentPaddingRight: '1.9rem',
      contentPaddingBottom: '1.55rem',
      contentPaddingLeft: '1.95rem',
    },
    chrome: {
      variant: 'writing',
      textColor: '#ffb558',
      accentColor: '#ffca7f',
      dimTextColor: '#d6964a',
      dividerColor: 'rgba(255, 198, 121, 0.86)',
      fileName: 'UNTITLED',
      modeLabel: 'WRITING',
      pageLabel: '01/01',
      commandField: '[    ]',
    },
    editorLayout: {
      fontFamily: '"Lucida Console", "Courier New", monospace',
      fontSize: 'clamp(1.05rem, 1.6vw, 2rem)',
      lineHeight: '1.42',
      letterSpacing: '0.032em',
      fontWeight: '400',
      placeholder: '',
      caretColor: '#ffca76',
      selectionColor: 'rgba(255, 190, 108, 0.18)',
      prompt: '>',
      promptColor: '#ffc763',
      paddingTop: '0.95rem',
      paddingRight: '0',
      paddingBottom: '0',
      paddingLeft: '1.8rem',
    },
    legend: {
      lines: [
        ['^G Help', '^O Save', '^K Cut', '^W Search', '^J Justify', '^C Status'],
        ['^X Exit', '', '^U Paste', '', '', ''],
      ],
    },
  },
  {
    id: 'starfleet-command',
    label: 'Starfleet Command',
    screenLayout: {
      scale: '0.86',
      safeInset: '1.2% 1.3% 1.8%',
    },
    screenFx: {
      appBackground: '#040610',
      roomGlow:
        'radial-gradient(circle at center, rgba(58, 84, 170, 0.28), rgba(10, 14, 28, 0.94) 56%, rgba(3, 5, 11, 1) 100%)',
      roomGridMajor: 'rgba(135, 192, 255, 0.028)',
      roomGridMinor: 'rgba(135, 192, 255, 0.016)',
      roomGridOpacity: '0.06',
      frameShadow:
        '0 0 0 1px rgba(113, 163, 255, 0.08), 0 0 90px rgba(0, 0, 0, 0.62)',
      monitorUnderlay: '#050713',
      screenInset: '8.5% 6.6% 11.2% 6.6%',
      screenBackground: '#080c1d',
      screenTint:
        'radial-gradient(circle at 50% 18%, rgba(94, 149, 255, 0.1), rgba(14, 17, 37, 0.82) 58%, rgba(7, 10, 22, 0.98) 100%)',
      screenVignette:
        'radial-gradient(circle at 50% 48%, rgba(0, 0, 0, 0) 60%, rgba(0, 0, 0, 0.36) 100%)',
      screenInnerShadow:
        'inset 0 0 52px rgba(0, 0, 0, 0.66), inset 0 0 20px rgba(96, 144, 255, 0.08)',
      scanline:
        'repeating-linear-gradient(to bottom, rgba(144, 191, 255, 0.026) 0, rgba(144, 191, 255, 0.026) 1px, transparent 2px, transparent 5px)',
      scanlineOpacity: '0.26',
      noiseOpacity: '0.08',
      textGlow: '0 0 10px rgba(255, 190, 113, 0.16)',
      contentPaddingTop: '0.95rem',
      contentPaddingRight: '0.95rem',
      contentPaddingBottom: '0.9rem',
      contentPaddingLeft: '0.95rem',
    },
    chrome: {
      variant: 'starfleet-command',
      textColor: '#f2b26e',
      accentColor: '#ffc56b',
      dimTextColor: '#8fcaff',
      dividerColor: 'rgba(255, 183, 109, 0.84)',
      tones: {
        peach: { background: '#f3a06c', textColor: '#11162d' },
        gold: { background: '#f4b841', textColor: '#11162d' },
        yellow: { background: '#edd66f', textColor: '#11162d' },
        orange: { background: '#ffbf68', textColor: '#11162d' },
        red: { background: '#dc625d', textColor: '#11162d' },
        blue: { background: '#3e95ff', textColor: '#101833' },
        sky: { background: '#81aefe', textColor: '#11162d' },
        cream: { background: '#ddd9a3', textColor: '#11162d' },
      },
      mastheadLines: [
        'UNITED FEDERATION OF PLANETS // STARFLEET COMMAND',
        'USS ENTERPRISE - NCC-1701-D // CONSOLE: OCP-04',
      ],
      title: 'WORD PROCESSOR / LOG ENTRY',
      topActions: [
        { label: 'SAVE', tone: 'orange' },
        { label: 'LOAD', tone: 'blue' },
        { label: 'NEW', tone: 'gold' },
        { label: 'PRINT/XFER', tone: 'yellow' },
        { label: 'EXIT', tone: 'red' },
      ],
      upperRails: [
        { tone: 'sky', flex: 8 },
        { tone: 'yellow', flex: 1.35 },
        { tone: 'blue', flex: 8.8 },
        { tone: 'orange', flex: 0.9 },
      ],
      lowerRails: [
        { tone: 'peach', flex: 7.4 },
        { tone: 'blue', flex: 1.1 },
        { tone: 'sky', flex: 7.6 },
        { tone: 'peach', flex: 2.2 },
      ],
      leftActions: [
        { label: 'FILE', tone: 'gold' },
        { label: 'EDIT', tone: 'blue' },
        { label: 'VIEW', tone: 'sky' },
        { label: 'INSERT', tone: 'yellow' },
        { label: 'FORMAT', tone: 'sky' },
        { label: 'TOOLS', tone: 'peach' },
      ],
      rightPanels: [
        { label: 'SYSTEM STATUS', tone: 'cream' },
        { label: 'COMMUNICATIONS', tone: 'peach' },
        { label: 'PERSONNEL DATA', tone: 'gold' },
        { label: 'NAVIGATION LOGS', tone: 'cream' },
      ],
      documentName: '[UNTITLED]',
      statusText: 'READY',
      editorHeading: 'STARSHIP LOG >',
      footerFields: [
        'ACTIVE: USER ALPHA-7',
        'DATE: STARDATE 47312.4',
        'LOC: BRIDGE CONSOLE',
        'SYSTEM: ONLINE',
      ],
    },
    editorLayout: {
      ...lcarsEditorLayout,
      caretColor: '#ffcf72',
      selectionColor: 'rgba(255, 197, 103, 0.18)',
      paddingTop: '0',
      paddingRight: '0',
      paddingBottom: '0',
      paddingLeft: '0',
    },
  },
  {
    id: 'enterprise-systems',
    label: 'Enterprise Systems',
    screenLayout: {
      scale: '0.84',
      safeInset: '1.2% 1.3% 2.1%',
    },
    screenFx: {
      appBackground: '#04050c',
      roomGlow:
        'radial-gradient(circle at center, rgba(86, 56, 128, 0.24), rgba(12, 11, 24, 0.94) 56%, rgba(5, 5, 12, 1) 100%)',
      roomGridMajor: 'rgba(255, 192, 130, 0.024)',
      roomGridMinor: 'rgba(167, 129, 255, 0.015)',
      roomGridOpacity: '0.05',
      frameShadow:
        '0 0 0 1px rgba(255, 172, 102, 0.08), 0 0 92px rgba(0, 0, 0, 0.64)',
      monitorUnderlay: '#080813',
      screenInset: '8.5% 6.6% 11.2% 6.6%',
      screenBackground: '#090c18',
      screenTint:
        'radial-gradient(circle at 50% 18%, rgba(255, 164, 99, 0.08), rgba(18, 20, 33, 0.82) 58%, rgba(9, 11, 20, 0.98) 100%)',
      screenVignette:
        'radial-gradient(circle at 50% 46%, rgba(0, 0, 0, 0) 60%, rgba(0, 0, 0, 0.38) 100%)',
      screenInnerShadow:
        'inset 0 0 52px rgba(0, 0, 0, 0.68), inset 0 0 16px rgba(168, 110, 255, 0.08)',
      scanline:
        'repeating-linear-gradient(to bottom, rgba(255, 183, 105, 0.024) 0, rgba(255, 183, 105, 0.024) 1px, transparent 2px, transparent 5px)',
      scanlineOpacity: '0.24',
      noiseOpacity: '0.08',
      textGlow: '0 0 9px rgba(255, 172, 104, 0.14)',
      contentPaddingTop: '0.9rem',
      contentPaddingRight: '0.9rem',
      contentPaddingBottom: '0.95rem',
      contentPaddingLeft: '0.9rem',
    },
    chrome: {
      variant: 'enterprise-systems',
      textColor: '#f2aa6d',
      accentColor: '#ffbe74',
      dimTextColor: '#c38bff',
      dividerColor: 'rgba(243, 171, 109, 0.8)',
      tones: {
        peach: { background: '#ef9d69', textColor: '#101427' },
        gold: { background: '#e2b427', textColor: '#101427' },
        yellow: { background: '#f1d57b', textColor: '#101427' },
        orange: { background: '#f7ba61', textColor: '#101427' },
        sky: { background: '#91abff', textColor: '#11162b' },
        violet: { background: '#a27fff', textColor: '#11162b' },
        cream: { background: '#f0d49d', textColor: '#11162b' },
      },
      headerLeft: 'USS ENTERPRISE',
      headerRight: 'COMPUTER SYSTEMS',
      subtitle: 'WORD PROCESSING INTERFACE',
      upperRails: [
        { tone: 'gold', flex: 0.7 },
        { tone: 'orange', flex: 1.1 },
        { tone: 'violet', flex: 3.2 },
      ],
      lowerRails: [
        { tone: 'violet', flex: 2.9 },
        { tone: 'peach', flex: 0.65 },
        { tone: 'orange', flex: 3.4 },
      ],
      primaryActions: [
        { label: 'LOAD FILE', tone: 'orange' },
        { label: 'SAVE DOCUMENT', tone: 'yellow' },
        { label: 'PRINT', tone: 'peach' },
      ],
      secondaryActions: [
        { label: 'DICTIONARY', tone: 'gold' },
        { label: 'THESAURUS', tone: 'orange' },
        { label: 'FORMAT OPTIONS', tone: 'cream' },
      ],
      documentName: '[UNTITLED]',
      stardate: '47812.5',
      footerMenu: ['FILE', 'EDIT', 'VIEW', 'INSERT', 'TOOLS', 'HELP'],
      loggedInAs: 'LOGGED AS: LT. COMMANDER GEORDI LA FORGE',
      terminal: 'TERMINAL 04-B',
      securityText: 'SECURE ACCESS ACTIVE',
      timeZone: 'PST',
    },
    editorLayout: {
      ...lcarsEditorLayout,
      fontSize: 'clamp(0.94rem, 1.18vw, 1.38rem)',
      caretColor: '#ffe049',
      selectionColor: 'rgba(255, 224, 73, 0.17)',
      paddingTop: '0.05rem',
      paddingRight: '0.15rem',
      paddingBottom: '0',
      paddingLeft: '0.15rem',
    },
  },
  {
    id: 'lcars-4-1',
    label: 'LCARS 4.1',
    screenLayout: {
      scale: '0.83',
      safeInset: '1.1% 1.3% 2.2%',
    },
    screenFx: {
      appBackground: '#04050f',
      roomGlow:
        'radial-gradient(circle at center, rgba(92, 141, 255, 0.22), rgba(11, 14, 29, 0.94) 54%, rgba(4, 5, 12, 1) 100%)',
      roomGridMajor: 'rgba(117, 183, 255, 0.028)',
      roomGridMinor: 'rgba(201, 133, 255, 0.015)',
      roomGridOpacity: '0.05',
      frameShadow:
        '0 0 0 1px rgba(118, 180, 255, 0.08), 0 0 94px rgba(0, 0, 0, 0.64)',
      monitorUnderlay: '#060813',
      screenInset: '8.5% 6.6% 11.2% 6.6%',
      screenBackground: '#070c1d',
      screenTint:
        'radial-gradient(circle at 50% 14%, rgba(116, 179, 255, 0.08), rgba(12, 18, 38, 0.82) 56%, rgba(7, 11, 24, 0.98) 100%)',
      screenVignette:
        'radial-gradient(circle at 50% 48%, rgba(0, 0, 0, 0) 61%, rgba(0, 0, 0, 0.36) 100%)',
      screenInnerShadow:
        'inset 0 0 54px rgba(0, 0, 0, 0.68), inset 0 0 20px rgba(132, 187, 255, 0.08)',
      scanline:
        'repeating-linear-gradient(to bottom, rgba(144, 197, 255, 0.024) 0, rgba(144, 197, 255, 0.024) 1px, transparent 2px, transparent 5px)',
      scanlineOpacity: '0.24',
      noiseOpacity: '0.08',
      textGlow: '0 0 9px rgba(255, 194, 88, 0.14)',
      contentPaddingTop: '0.85rem',
      contentPaddingRight: '0.9rem',
      contentPaddingBottom: '0.95rem',
      contentPaddingLeft: '0.9rem',
    },
    chrome: {
      variant: 'lcars-4-1',
      textColor: '#a9ccff',
      accentColor: '#ffbd55',
      dimTextColor: '#76d0ff',
      dividerColor: 'rgba(131, 192, 255, 0.76)',
      tones: {
        orange: { background: '#ff9961', textColor: '#10152c' },
        gold: { background: '#efcc65', textColor: '#10152c' },
        yellow: { background: '#f2dc73', textColor: '#10152c' },
        sky: { background: '#78bcff', textColor: '#10152c' },
        blue: { background: '#6498ff', textColor: '#10152c' },
        violet: { background: '#9892ff', textColor: '#10152c' },
        pink: { background: '#c58dde', textColor: '#10152c' },
        mint: { background: '#8fd6a1', textColor: '#10152c' },
        red: { background: '#e36163', textColor: '#10152c' },
      },
      title: 'LCARS 4.1 WORD PROCESSOR',
      commandLine:
        'STARFLEET COMMAND / USS ENTERPRISE NCC-1701-D / DOCUMENT EDITOR',
      metaFields: [
        'FILE: [NEW_LOG]',
        'STATUS: ACTIVE',
        'AUTHOR: [PENDING]',
        'DATE: 2384.110',
      ],
      systemStatus: '[SYSTEM STATUS: NOMINAL]',
      networkStatus: '[NETWORK: ACTIVE]',
      editorTitle: '** [UNTITLED LOG] **',
      leftActions: [
        { label: '[CREATE]', tone: 'orange' },
        { label: '[OPEN]', tone: 'gold' },
        { label: '[SAVE]', tone: 'violet' },
        { label: '[PRINT]', tone: 'pink' },
        { label: '[FIND]', tone: 'sky' },
        { label: '[EDIT]', tone: 'blue' },
      ],
      rightActions: [
        { label: '[BOLD]', tone: 'yellow' },
        { label: '[ITALIC]', tone: 'violet' },
        { label: '[UNDERLINE]', tone: 'pink' },
        { label: '[FONT]', tone: 'mint' },
        { label: '[INSERT]', tone: 'orange' },
        { label: '[DELETE]', tone: 'red' },
      ],
      bottomActions: [
        { label: '[FORMAT]', tone: 'sky' },
        { label: '[SPELL]', tone: 'blue' },
        { label: '[TRANS]', tone: 'violet' },
        { label: '[ENCRYPT]', tone: 'mint' },
        { label: '[CLOSE]', tone: 'red' },
        { label: '[HELP]', tone: 'orange' },
        { label: '[EXIT]', tone: 'gold' },
      ],
      fileFooter: '[FILE: NEW]',
    },
    editorLayout: {
      ...lcarsEditorLayout,
      fontSize: 'clamp(0.94rem, 1.18vw, 1.38rem)',
      caretColor: '#ffbe58',
      selectionColor: 'rgba(255, 190, 88, 0.18)',
      paddingTop: '0',
      paddingRight: '0.1rem',
      paddingBottom: '0',
      paddingLeft: '0.1rem',
    },
  },
  {
    id: 'mother-6800',
    label: 'MU/TH/UR 6800',
    screenLayout: {
      scale: '0.92',
      safeInset: '1.3% 1.4% 2.2%',
    },
    screenFx: {
      appBackground: '#040603',
      roomGlow:
        'radial-gradient(circle at center, rgba(52, 93, 38, 0.3), rgba(8, 13, 7, 0.94) 58%, rgba(2, 4, 2, 1) 100%)',
      roomGridMajor: 'rgba(139, 255, 142, 0.03)',
      roomGridMinor: 'rgba(139, 255, 142, 0.016)',
      roomGridOpacity: '0.07',
      frameShadow:
        '0 0 0 1px rgba(121, 255, 143, 0.08), 0 0 86px rgba(0, 0, 0, 0.6)',
      monitorUnderlay: '#030502',
      screenInset: '8.5% 6.6% 11.2% 6.6%',
      screenBackground: '#031206',
      screenTint:
        'radial-gradient(circle at 50% 24%, rgba(102, 255, 132, 0.11), rgba(8, 22, 11, 0.94) 60%, rgba(2, 9, 4, 0.98) 100%)',
      screenVignette:
        'radial-gradient(circle at 50% 48%, rgba(0, 0, 0, 0) 58%, rgba(0, 0, 0, 0.46) 100%)',
      screenInnerShadow:
        'inset 0 0 46px rgba(0, 0, 0, 0.68), inset 0 0 20px rgba(76, 186, 92, 0.08)',
      scanline:
        'repeating-linear-gradient(to bottom, rgba(136, 255, 154, 0.038) 0, rgba(136, 255, 154, 0.038) 1px, transparent 2px, transparent 4px)',
      scanlineOpacity: '0.34',
      noiseOpacity: '0.12',
      textGlow: '0 0 9px rgba(132, 255, 144, 0.16)',
      contentPaddingTop: '0.95rem',
      contentPaddingRight: '1rem',
      contentPaddingBottom: '0.95rem',
      contentPaddingLeft: '1rem',
    },
    chrome: {
      variant: 'mother-6800',
      textColor: '#83fa8a',
      accentColor: '#a2ffaa',
      dimTextColor: '#62d66e',
      dividerColor: 'rgba(132, 255, 151, 0.82)',
      inverseTextColor: '#09240d',
      inverseBackground: '#9ff7a6',
      systemName: 'MU/TH/UR 6800',
      consoleName: 'CONSOLE 3 // MP_v4.2',
      documentName: '[EMPTY]',
      pageLabel: '001',
      modeLabel: 'AUTO',
      menuItems: ['FILE', 'EDIT', 'VIEW', 'INSERT', 'FORMAT', 'TOOLS', 'WINDOW', 'HELP'],
      railItems: Array.from({ length: 20 }, (_, index) =>
        `${(index + 1).toString().padStart(2, '0')}>`,
      ),
      promptLabel: 'INPUT BUFFER //',
      footerLeft: 'SYS LINK: PRIMARY',
      footerRight: 'TERMINAL READY',
    },
    editorLayout: {
      ...alienEditorLayout,
      fontSize: 'clamp(0.92rem, 1.12vw, 1.24rem)',
      letterSpacing: '0.042em',
      caretColor: '#aaff9f',
      selectionColor: 'rgba(170, 255, 159, 0.16)',
      paddingTop: '0.1rem',
      paddingRight: '0.15rem',
      paddingBottom: '0.2rem',
      paddingLeft: '0',
    },
  },
  {
    id: 'ready-input',
    label: 'Ready for Input',
    screenLayout: {
      scale: '0.93',
      safeInset: '1.2% 1.4% 2.3%',
    },
    screenFx: {
      appBackground: '#040603',
      roomGlow:
        'radial-gradient(circle at center, rgba(44, 90, 40, 0.3), rgba(7, 13, 7, 0.94) 58%, rgba(2, 4, 2, 1) 100%)',
      roomGridMajor: 'rgba(139, 255, 142, 0.025)',
      roomGridMinor: 'rgba(139, 255, 142, 0.014)',
      roomGridOpacity: '0.06',
      frameShadow:
        '0 0 0 1px rgba(121, 255, 143, 0.07), 0 0 84px rgba(0, 0, 0, 0.58)',
      monitorUnderlay: '#030502',
      screenInset: '8.5% 6.6% 11.2% 6.6%',
      screenBackground: '#021003',
      screenTint:
        'radial-gradient(circle at 50% 22%, rgba(94, 255, 122, 0.09), rgba(7, 20, 8, 0.92) 58%, rgba(2, 8, 3, 0.98) 100%)',
      screenVignette:
        'radial-gradient(circle at 50% 48%, rgba(0, 0, 0, 0) 60%, rgba(0, 0, 0, 0.44) 100%)',
      screenInnerShadow:
        'inset 0 0 42px rgba(0, 0, 0, 0.66), inset 0 0 18px rgba(82, 180, 88, 0.07)',
      scanline:
        'repeating-linear-gradient(to bottom, rgba(136, 255, 154, 0.034) 0, rgba(136, 255, 154, 0.034) 1px, transparent 2px, transparent 4px)',
      scanlineOpacity: '0.3',
      noiseOpacity: '0.1',
      textGlow: '0 0 8px rgba(136, 255, 154, 0.14)',
      contentPaddingTop: '1rem',
      contentPaddingRight: '1.1rem',
      contentPaddingBottom: '1rem',
      contentPaddingLeft: '1.1rem',
    },
    chrome: {
      variant: 'ready-input',
      textColor: '#87ff8f',
      accentColor: '#abffac',
      dimTextColor: '#67d26d',
      dividerColor: 'rgba(139, 255, 142, 0.8)',
      documentName: '[UNTITLED]',
      statusLabel: 'READY FOR INPUT',
      pageLabel: '01',
      commandHints: [
        'F1: HELP',
        'F2: SAVE',
        'F3: OPEN',
        'F4: CLOSE',
        'F5: PRINT',
        'F6: OPTIONS',
      ],
      footerHint: 'CTRL+Q: QUIT',
    },
    editorLayout: {
      ...alienEditorLayout,
      fontSize: 'clamp(1rem, 1.26vw, 1.42rem)',
      lineHeight: '1.5',
      letterSpacing: '0.04em',
      caretColor: '#b0ffb1',
      selectionColor: 'rgba(176, 255, 177, 0.15)',
      paddingTop: '0.15rem',
      paddingRight: '0.1rem',
      paddingBottom: '0.15rem',
      paddingLeft: '0.1rem',
    },
  },
  {
    id: 'nostromo-data-systems',
    label: 'Nostromo Data Systems',
    screenLayout: {
      scale: '0.91',
      safeInset: '1.3% 1.5% 2.3%',
    },
    screenFx: {
      appBackground: '#040603',
      roomGlow:
        'radial-gradient(circle at center, rgba(42, 82, 34, 0.28), rgba(8, 12, 7, 0.95) 56%, rgba(2, 4, 2, 1) 100%)',
      roomGridMajor: 'rgba(174, 226, 160, 0.022)',
      roomGridMinor: 'rgba(174, 226, 160, 0.012)',
      roomGridOpacity: '0.05',
      frameShadow:
        '0 0 0 1px rgba(166, 219, 151, 0.06), 0 0 88px rgba(0, 0, 0, 0.62)',
      monitorUnderlay: '#040502',
      screenInset: '8.5% 6.6% 11.2% 6.6%',
      screenBackground: '#090d06',
      screenTint:
        'radial-gradient(circle at 50% 18%, rgba(195, 224, 149, 0.08), rgba(18, 22, 12, 0.92) 58%, rgba(8, 9, 4, 0.98) 100%)',
      screenVignette:
        'radial-gradient(circle at 50% 48%, rgba(0, 0, 0, 0) 60%, rgba(0, 0, 0, 0.48) 100%)',
      screenInnerShadow:
        'inset 0 0 48px rgba(0, 0, 0, 0.7), inset 0 0 18px rgba(186, 214, 144, 0.06)',
      scanline:
        'repeating-linear-gradient(to bottom, rgba(198, 236, 164, 0.026) 0, rgba(198, 236, 164, 0.026) 1px, transparent 2px, transparent 5px)',
      scanlineOpacity: '0.26',
      noiseOpacity: '0.1',
      textGlow: '0 0 8px rgba(214, 230, 154, 0.12)',
      contentPaddingTop: '1rem',
      contentPaddingRight: '1.05rem',
      contentPaddingBottom: '1rem',
      contentPaddingLeft: '1.05rem',
    },
    chrome: {
      variant: 'nostromo-data-systems',
      textColor: '#c7e4a1',
      accentColor: '#d8efad',
      dimTextColor: '#9fb980',
      dividerColor: 'rgba(210, 236, 168, 0.74)',
      headerTitle: 'NOSTROMO DATA SYSTEMS',
      accessLabel: 'SYSTEM ACCESS: [LOGGED: RIPLEY, E.]',
      editorStatus: 'TEXT EDITOR v3.14 / ACTIVE',
      documentName: 'EMPTY',
      modeLabel: 'WRITE',
      insertLabel: 'INS',
      wrapLabel: 'WORD WRAP: ON',
      memoryLabel: '243 KB FREE',
      commandHints: [
        'F1: SAVE',
        'F2: OPEN',
        'F3: QUIT',
        'F4: FIND',
        'F5: PRINT',
        'F6: OPTIONS',
      ],
      footerHint: '<ENTER> TO START',
    },
    editorLayout: {
      ...alienEditorLayout,
      fontSize: 'clamp(0.94rem, 1.18vw, 1.3rem)',
      lineHeight: '1.44',
      letterSpacing: '0.04em',
      caretColor: '#dceda8',
      selectionColor: 'rgba(220, 237, 168, 0.15)',
      paddingTop: '0.05rem',
      paddingRight: '0.1rem',
      paddingBottom: '0.15rem',
      paddingLeft: '0.1rem',
    },
  },
  {
    id: 'c64-speedwrite',
    label: 'C64 SpeedWrite',
    screenLayout: {
      scale: '0.92',
      safeInset: '1.6% 1.8% 2.6%',
    },
    screenFx: {
      ...c64ScreenFxBase,
      appBackground: '#070a1d',
      roomGlow:
        'radial-gradient(circle at center, rgba(119, 149, 255, 0.3), rgba(19, 25, 66, 0.94) 56%, rgba(5, 8, 22, 1) 100%)',
      monitorUnderlay: '#090d28',
      screenBackground: '#2f408e',
      screenTint:
        'radial-gradient(circle at 50% 24%, rgba(198, 222, 255, 0.08), rgba(55, 72, 152, 0.9) 58%, rgba(34, 48, 118, 0.98) 100%)',
      scanlineOpacity: '0.2',
      noiseOpacity: '0.07',
      contentPaddingTop: '0.52rem',
      contentPaddingRight: '0.72rem',
      contentPaddingBottom: '0.62rem',
      contentPaddingLeft: '0.7rem',
    },
    chrome: {
      variant: 'c64-speedwrite',
      textColor: '#b8d8ff',
      accentColor: '#c8e2ff',
      dimTextColor: '#8eb5ff',
      dividerColor: 'rgba(196, 222, 255, 0.88)',
      inverseTextColor: '#29438d',
      inverseBackground: '#bddbff',
      title: '** SPEEDWRITE 64 **',
      documentName: 'UNTITLED',
      modeLabel: 'INSERT',
      footerHints: ['F1-HELP', 'F3-FILE', 'F5-EDIT', 'F7-FORMAT', 'CTRL-X EXIT'],
    },
    editorLayout: {
      ...c64EditorLayout,
      fontSize: 'clamp(0.96rem, 1.14vw, 1.3rem)',
      lineHeight: '1.28',
      letterSpacing: '0.055em',
      caretColor: '#cfe8ff',
      selectionColor: 'rgba(209, 232, 255, 0.18)',
      paddingTop: '0.06rem',
      paddingRight: '0.1rem',
      paddingBottom: '0',
      paddingLeft: '0.12rem',
      gutter: {
        lineCount: 24,
        width: '1rem',
        gap: '0.58rem',
        color: '#9bc1ff',
        activeColor: '#d7ebff',
        fontSize: 'clamp(0.84rem, 0.96vw, 1.04rem)',
        formatter: (lineNumber: number) => lineNumber.toString(),
      },
    },
  },
  {
    id: 'c64-easycript',
    label: 'C64 EasyScript',
    screenLayout: {
      scale: '0.9',
      safeInset: '1.6% 1.9% 2.8%',
    },
    screenFx: {
      ...c64ScreenFxBase,
      appBackground: '#050814',
      roomGlow:
        'radial-gradient(circle at center, rgba(104, 140, 255, 0.26), rgba(12, 18, 54, 0.96) 58%, rgba(4, 7, 18, 1) 100%)',
      frameShadow:
        '0 0 0 1px rgba(168, 202, 255, 0.08), 0 0 96px rgba(0, 0, 0, 0.68)',
      monitorUnderlay: '#060922',
      screenBackground: '#2d3f95',
      screenTint:
        'radial-gradient(circle at 50% 18%, rgba(192, 221, 255, 0.07), rgba(53, 69, 150, 0.88) 56%, rgba(33, 44, 118, 0.98) 100%)',
      contentPaddingTop: '0.46rem',
      contentPaddingRight: '0.68rem',
      contentPaddingBottom: '0.5rem',
      contentPaddingLeft: '0.68rem',
    },
    chrome: {
      variant: 'c64-easycript',
      textColor: '#b7d6ff',
      accentColor: '#cae3ff',
      dimTextColor: '#98bcff',
      dividerColor: 'rgba(198, 224, 255, 0.88)',
      inverseTextColor: '#273f87',
      inverseBackground: '#c0dcff',
      title: '*** EASYSCRIPT 64 V1.2 ***',
      documentName: '<NEW>',
      modeLabel: 'INS',
    },
    editorLayout: {
      ...c64EditorLayout,
      fontSize: 'clamp(0.98rem, 1.16vw, 1.34rem)',
      lineHeight: '1.3',
      letterSpacing: '0.055em',
      caretColor: '#d2e9ff',
      selectionColor: 'rgba(210, 233, 255, 0.18)',
      paddingTop: '0.18rem',
      paddingRight: '0.18rem',
      paddingBottom: '0',
      paddingLeft: '0.18rem',
    },
  },
  {
    id: 'c64-writer',
    label: 'C64 Writer',
    screenLayout: {
      scale: '0.9',
      safeInset: '1.6% 1.9% 2.6%',
    },
    screenFx: {
      ...c64ScreenFxBase,
      appBackground: '#060a1b',
      roomGlow:
        'radial-gradient(circle at center, rgba(114, 149, 255, 0.28), rgba(17, 24, 64, 0.94) 56%, rgba(5, 8, 19, 1) 100%)',
      monitorUnderlay: '#080b24',
      screenBackground: '#374ca3',
      screenTint:
        'radial-gradient(circle at 50% 14%, rgba(215, 232, 255, 0.06), rgba(78, 102, 197, 0.34) 42%, rgba(50, 69, 151, 0.96) 100%)',
      textGlow: '0 0 6px rgba(36, 52, 123, 0.12)',
      contentPaddingTop: '0.5rem',
      contentPaddingRight: '0.7rem',
      contentPaddingBottom: '0.66rem',
      contentPaddingLeft: '0.7rem',
    },
    chrome: {
      variant: 'c64-writer',
      textColor: '#2d4291',
      accentColor: '#233983',
      dimTextColor: '#4e69ba',
      dividerColor: 'rgba(49, 71, 164, 0.82)',
      inverseTextColor: '#233983',
      inverseBackground: '#c8defd',
      title: 'C64 WRITER v1.2',
      documentName: 'UNTITLED',
      bottomBarLabel: 'READY',
    },
    editorLayout: {
      ...c64EditorLayout,
      fontSize: 'clamp(1rem, 1.2vw, 1.38rem)',
      lineHeight: '1.32',
      letterSpacing: '0.05em',
      caretColor: '#2c408f',
      selectionColor: 'rgba(65, 91, 183, 0.18)',
      paddingTop: '0.22rem',
      paddingRight: '0.24rem',
      paddingBottom: '0',
      paddingLeft: '0.22rem',
    },
  },
  {
    id: 'c64-function-bar',
    label: 'C64 Function Bar',
    screenLayout: {
      scale: '0.92',
      safeInset: '1.4% 1.5% 2.4%',
    },
    screenFx: {
      ...c64ScreenFxBase,
      appBackground: '#04070f',
      roomGlow:
        'radial-gradient(circle at center, rgba(54, 99, 171, 0.24), rgba(10, 15, 34, 0.95) 58%, rgba(4, 7, 16, 1) 100%)',
      roomGridMajor: 'rgba(105, 190, 255, 0.022)',
      roomGridMinor: 'rgba(105, 190, 255, 0.012)',
      frameShadow:
        '0 0 0 1px rgba(94, 175, 255, 0.08), 0 0 92px rgba(0, 0, 0, 0.68)',
      monitorUnderlay: '#060a18',
      screenBackground: '#10183d',
      screenTint:
        'radial-gradient(circle at 50% 16%, rgba(82, 160, 255, 0.06), rgba(20, 29, 69, 0.84) 56%, rgba(9, 14, 35, 0.98) 100%)',
      screenInnerShadow:
        'inset 0 0 46px rgba(0, 0, 0, 0.7), inset 0 0 18px rgba(98, 180, 255, 0.06)',
      scanline:
        'repeating-linear-gradient(to bottom, rgba(126, 197, 255, 0.022) 0, rgba(126, 197, 255, 0.022) 1px, transparent 2px, transparent 5px)',
      scanlineOpacity: '0.24',
      noiseOpacity: '0.08',
      textGlow: '0 0 8px rgba(102, 192, 255, 0.12)',
      contentPaddingTop: '0.56rem',
      contentPaddingRight: '0.74rem',
      contentPaddingBottom: '0.68rem',
      contentPaddingLeft: '0.74rem',
    },
    chrome: {
      variant: 'c64-function-bar',
      textColor: '#8cd7ff',
      accentColor: '#7fd8ff',
      dimTextColor: '#e4ca57',
      dividerColor: 'rgba(118, 205, 255, 0.88)',
      commandHints: [
        '[F1]=MENU',
        '[F3]=LOAD',
        '[F5]=SAVE',
        '[F7]=QUIT',
        '[INS]=DEL',
        '[HOME]=TOP',
        '[CLR]=NEW',
      ],
      documentName: '(Untitled)',
      capsLabel: 'OFF',
      modifiedLabel: 'NO',
      separatorText:
        '--------------------------------------------------------------------------',
    },
    editorLayout: {
      ...c64EditorLayout,
      fontSize: 'clamp(0.94rem, 1.12vw, 1.24rem)',
      lineHeight: '1.36',
      letterSpacing: '0.045em',
      caretColor: '#4ff2ff',
      selectionColor: 'rgba(102, 211, 255, 0.18)',
      paddingTop: '0.08rem',
      paddingRight: '0.12rem',
      paddingBottom: '0',
      paddingLeft: '0.12rem',
    },
  },
  {
    id: 'appleworks-1-1',
    label: 'AppleWorks 1.1',
    screenLayout: {
      scale: '0.93',
      safeInset: '1.6% 1.7% 2.4%',
    },
    screenFx: {
      ...appleScreenFxBase,
      contentPaddingTop: '0.78rem',
      contentPaddingRight: '0.88rem',
      contentPaddingBottom: '0.82rem',
      contentPaddingLeft: '0.88rem',
    },
    chrome: {
      variant: 'appleworks-menu',
      textColor: '#73f48a',
      accentColor: '#97ffad',
      dimTextColor: '#5ecb73',
      dividerColor: 'rgba(128, 255, 150, 0.82)',
      mastheadLeft: 'APPLEWORKS 1.1',
      mastheadCenter: 'WORD PROCESSOR',
      mastheadRight: '(ESC) MAIN MENU',
      menuItems: [
        'FILES: (1) Add (2) Remove (3) List',
        'EDIT: (4) Move (5) Copy (6) Print',
      ],
      fileLabel: 'FILENAME: UNTITLED',
      pageLabel: 'PAGE  1 OF  1',
      lineLabel: 'LINE  1',
    },
    editorLayout: {
      ...alienEditorLayout,
      fontFamily: '"Lucida Console", "Courier New", monospace',
      fontSize: 'clamp(0.96rem, 1.12vw, 1.24rem)',
      lineHeight: '1.36',
      letterSpacing: '0.028em',
      caretColor: '#9fffaf',
      selectionColor: 'rgba(155, 255, 171, 0.16)',
      paddingTop: '0.08rem',
      paddingRight: '0',
      paddingBottom: '0',
      paddingLeft: '0',
      gutter: {
        lineCount: 24,
        width: '2.45rem',
        gap: '0.52rem',
        color: '#6ad57c',
        activeColor: '#a3ffb4',
        fontSize: 'clamp(0.88rem, 1.02vw, 1.12rem)',
        formatter: (lineNumber: number) => lineNumber.toString().padStart(2, ' '),
      },
    },
  },
  {
    id: 'apple-writer-ii',
    label: 'Apple Writer II',
    screenLayout: {
      scale: '0.94',
      safeInset: '1.5% 1.7% 2.3%',
    },
    screenFx: {
      ...appleScreenFxBase,
      screenBackground: '#021003',
      screenTint:
        'radial-gradient(circle at 50% 24%, rgba(95, 255, 124, 0.1), rgba(7, 18, 7, 0.92) 58%, rgba(2, 8, 3, 0.98) 100%)',
      contentPaddingTop: '0.72rem',
      contentPaddingRight: '0.78rem',
      contentPaddingBottom: '0.8rem',
      contentPaddingLeft: '0.8rem',
    },
    chrome: {
      variant: 'applewriter-family',
      textColor: '#76f58a',
      accentColor: '#97ffac',
      dimTextColor: '#61ce75',
      dividerColor: 'rgba(122, 255, 148, 0.78)',
      titleLine: 'APPLE WRITER II [DOCUMENT: UNTITLED]  LN:01 COL:01 [NEW FILE]',
      titleAlign: 'left',
    },
    editorLayout: {
      ...alienEditorLayout,
      fontFamily: '"Lucida Console", "Courier New", monospace',
      fontSize: 'clamp(0.98rem, 1.18vw, 1.3rem)',
      lineHeight: '1.38',
      letterSpacing: '0.03em',
      caretColor: '#a5ffb2',
      selectionColor: 'rgba(157, 255, 173, 0.16)',
      paddingTop: '0.1rem',
      paddingRight: '0',
      paddingBottom: '0',
      paddingLeft: '0',
      gutter: {
        lineCount: 24,
        width: '2.38rem',
        gap: '0.48rem',
        color: '#69d47b',
        activeColor: '#a2ffb4',
        fontSize: 'clamp(0.88rem, 1.04vw, 1.16rem)',
        formatter: (lineNumber: number) => lineNumber.toString().padStart(2, ' '),
      },
    },
  },
  {
    id: 'apple-ii-word-processor',
    label: 'Apple II Word Processor',
    screenLayout: {
      scale: '0.94',
      safeInset: '1.4% 1.7% 2.2%',
    },
    screenFx: {
      ...appleScreenFxBase,
      screenTint:
        'radial-gradient(circle at 50% 20%, rgba(121, 255, 151, 0.14), rgba(10, 27, 12, 0.94) 62%, rgba(3, 10, 4, 0.98) 100%)',
      scanlineOpacity: '0.38',
      contentPaddingTop: '0.68rem',
      contentPaddingRight: '0.76rem',
      contentPaddingBottom: '0.74rem',
      contentPaddingLeft: '0.76rem',
    },
    chrome: {
      variant: 'banner-footer',
      textColor: '#7dfa9b',
      accentColor: '#a5ffbb',
      dimTextColor: '#63ce7f',
      dividerColor: 'rgba(144, 255, 166, 0.8)',
      inverseTextColor: '#103116',
      inverseBackground: '#9dfaae',
      headerLines: [
        { text: 'APPLE II WORD PROCESSOR v1.0', align: 'center', inverse: true },
        {
          text: 'FILE: [UNTITLED]     SIZE: 8K     FREE: 48K',
          align: 'center',
        },
      ],
      footerLines: [
        { text: 'LINE 1   COL 1 | CTRL+@=MENU', align: 'center' },
        {
          text: 'CTRL+O=OPEN   CTRL+S=SAVE   CTRL+P=PRINT',
          align: 'center',
        },
      ],
      dividerAfterHeader: true,
      dividerBeforeFooter: false,
    },
    editorLayout: {
      ...alienEditorLayout,
      fontFamily: '"Lucida Console", "Courier New", monospace',
      fontSize: 'clamp(1rem, 1.2vw, 1.34rem)',
      lineHeight: '1.4',
      letterSpacing: '0.03em',
      caretColor: '#a9ffb8',
      selectionColor: 'rgba(169, 255, 184, 0.16)',
      paddingTop: '0.15rem',
      paddingRight: '0',
      paddingBottom: '0',
      paddingLeft: '0',
    },
  },
  {
    id: 'applewriter-ii-v1-1',
    label: 'AppleWriter II v1.1',
    screenLayout: {
      scale: '0.93',
      safeInset: '1.6% 1.8% 2.4%',
    },
    screenFx: {
      ...appleScreenFxBase,
      roomGlow:
        'radial-gradient(circle at center, rgba(52, 108, 61, 0.34), rgba(8, 13, 8, 0.95) 56%, rgba(2, 4, 2, 1) 100%)',
      contentPaddingTop: '0.76rem',
      contentPaddingRight: '0.88rem',
      contentPaddingBottom: '0.8rem',
      contentPaddingLeft: '0.88rem',
    },
    chrome: {
      variant: 'applewriter-family',
      textColor: '#7af490',
      accentColor: '#9cffb1',
      dimTextColor: '#65cf78',
      dividerColor: 'rgba(135, 255, 156, 0.82)',
      titleLine: 'APPLEWRITER II - VERS 1.1 (C) APPLE 1982',
      titleAlign: 'center',
      statusLine: 'LINE: 1  COL: 1  MEM: 12423  MODE: INS.  FILE: UNTITLED',
      statusAlign: 'left',
    },
    editorLayout: {
      ...alienEditorLayout,
      fontFamily: '"Lucida Console", "Courier New", monospace',
      fontSize: 'clamp(0.96rem, 1.14vw, 1.26rem)',
      lineHeight: '1.36',
      letterSpacing: '0.03em',
      caretColor: '#a1ffb2',
      selectionColor: 'rgba(161, 255, 178, 0.16)',
      paddingTop: '0.08rem',
      paddingRight: '0',
      paddingBottom: '0',
      paddingLeft: '0',
      gutter: {
        lineCount: 24,
        width: '2.6rem',
        gap: '0.48rem',
        color: '#6bd07b',
        activeColor: '#a5ffb4',
        fontSize: 'clamp(0.88rem, 1.04vw, 1.16rem)',
        formatter: (lineNumber: number) => `${lineNumber.toString().padStart(2, ' ')}.`,
      },
    },
  },
  {
    id: 'amber-editor',
    label: 'Amber Editor',
    screenLayout: {
      scale: '0.94',
      safeInset: '1.4% 1.7% 2.4%',
    },
    screenFx: {
      ...amberScreenFxBase,
      contentPaddingTop: '0.82rem',
      contentPaddingRight: '0.86rem',
      contentPaddingBottom: '0.88rem',
      contentPaddingLeft: '0.86rem',
    },
    chrome: {
      variant: 'banner-footer',
      textColor: '#ffb95b',
      accentColor: '#ffd47d',
      dimTextColor: '#d38e42',
      dividerColor: 'rgba(255, 202, 129, 0.84)',
      inverseTextColor: '#2f1403',
      inverseBackground: '#ffca72',
      headerLines: [
        {
          text: '[EDITOR] document.txt  [LINES: 1]  [MODIFIED: N]  [READY]  11:34 AM',
        },
      ],
      footerLines: [
        {
          text: '^N: Save  ^X: Exit  ^K: Cut  ^U: Uncut  ^J: Help  ^O: Options  ^G: GoTo',
          align: 'center',
        },
      ],
      dividerAfterHeader: true,
      dividerBeforeFooter: true,
    },
    editorLayout: {
      ...alienEditorLayout,
      fontFamily: '"Lucida Console", "Courier New", monospace',
      fontSize: 'clamp(1rem, 1.16vw, 1.28rem)',
      lineHeight: '1.42',
      letterSpacing: '0.032em',
      caretColor: '#ffcf76',
      selectionColor: 'rgba(255, 198, 114, 0.16)',
      paddingTop: '0.1rem',
      paddingRight: '0',
      paddingBottom: '0',
      paddingLeft: '0',
    },
  },
  {
    id: 'nwriter-amber',
    label: 'NWriter Amber',
    screenLayout: {
      scale: '0.95',
      safeInset: '1.4% 1.7% 2.5%',
    },
    screenFx: {
      ...amberScreenFxBase,
      contentPaddingTop: '0.74rem',
      contentPaddingRight: '0.78rem',
      contentPaddingBottom: '0.84rem',
      contentPaddingLeft: '0.78rem',
    },
    chrome: {
      variant: 'framed-terminal',
      textColor: '#ffb85a',
      accentColor: '#ffd37b',
      dimTextColor: '#d99245',
      dividerColor: 'rgba(255, 195, 122, 0.86)',
      inverseTextColor: '#341704',
      inverseBackground: '#ffc15f',
      headerLeft: 'NWRITER - V2.1 [AMBER EDITION]',
      headerCenter: 'DOC: UNTITLED1.TXT',
      headerRight: 'LINE: 1   COL: 1   14:32',
      statusLabel: '* [Edit] *',
      commandHints: [
        '[F1] HELP',
        '[F2] FILE',
        '[F3] EDIT',
        '[F4] SEARCH',
        '[F5] FORMAT',
        '[F6] TOOLS',
        '[ESC] QUIT',
      ],
    },
    editorLayout: {
      ...alienEditorLayout,
      fontFamily: '"Lucida Console", "Courier New", monospace',
      fontSize: 'clamp(0.98rem, 1.14vw, 1.26rem)',
      lineHeight: '1.4',
      letterSpacing: '0.032em',
      caretColor: '#ffcd73',
      selectionColor: 'rgba(255, 199, 118, 0.16)',
      paddingTop: '0.18rem',
      paddingRight: '0.28rem',
      paddingBottom: '0.18rem',
      paddingLeft: '0.28rem',
    },
  },
  {
    id: 'uw-unix-nano',
    label: 'UW-UNIX Nano',
    screenLayout: {
      scale: '0.95',
      safeInset: '1.2% 1.5% 2.2%',
    },
    screenFx: {
      ...amberScreenFxBase,
      screenBackground: '#090402',
      contentPaddingTop: '0.62rem',
      contentPaddingRight: '0.66rem',
      contentPaddingBottom: '0.62rem',
      contentPaddingLeft: '0.66rem',
    },
    chrome: {
      variant: 'banner-footer',
      textColor: '#ffbc65',
      accentColor: '#ffd687',
      dimTextColor: '#d69043',
      dividerColor: 'rgba(255, 199, 126, 0.82)',
      inverseTextColor: '#2d1202',
      inverseBackground: '#ffcc74',
      headerLines: [
        {
          text: 'UW-UNIX v4.3 [1989] | nano 3.2  NEW BUFFER (empty.txt)',
          inverse: true,
        },
      ],
      footerLines: [
        {
          text: '^G Help   ^O WriteOut   ^R Read File   ^Y Prev Pg   ^K Cut Text   ^C Cur Pos',
          inverse: true,
        },
        {
          text: '^X Exit   ^J Justify   ^W Where Is   ^V Next Pg   ^U UnCut Text   ^T To Spell',
          inverse: true,
        },
      ],
    },
    editorLayout: {
      ...alienEditorLayout,
      fontFamily: '"Lucida Console", "Courier New", monospace',
      fontSize: 'clamp(1rem, 1.16vw, 1.3rem)',
      lineHeight: '1.44',
      letterSpacing: '0.03em',
      caretColor: '#ffd278',
      selectionColor: 'rgba(255, 205, 128, 0.16)',
      paddingTop: '0.16rem',
      paddingRight: '0',
      paddingBottom: '0',
      paddingLeft: '0',
    },
  },
  {
    id: 'unix-vi-emacs',
    label: 'UNIX VI Emacs',
    screenLayout: {
      scale: '0.94',
      safeInset: '1.4% 1.7% 2.4%',
    },
    screenFx: {
      ...amberScreenFxBase,
      screenTint:
        'radial-gradient(circle at 50% 24%, rgba(255, 173, 78, 0.1), rgba(76, 31, 9, 0.22) 42%, rgba(18, 8, 3, 0.98) 100%)',
      contentPaddingTop: '0.74rem',
      contentPaddingRight: '0.86rem',
      contentPaddingBottom: '0.82rem',
      contentPaddingLeft: '0.86rem',
    },
    chrome: {
      variant: 'unix-status',
      textColor: '#ffbc62',
      accentColor: '#ffd481',
      dimTextColor: '#d28e43',
      dividerColor: 'rgba(255, 198, 124, 0.84)',
      titleLine: 'UNIX VI (TM) VERSION 3.1 [EMACS MODE]    FILE: /usr/people/drh/doc/notes.txt',
      metaLine: 'MODES: INSERT | DATE: 12-OCT-88 | TIME: 11:42:36',
      footerLeft: '[ESC]=COMMAND  [Ctrl+X]=SAVE  [Ctrl+C]=EXIT  [Ctrl+Z]=UNDO',
      footerRight: 'LINE: 1  COL: 1',
    },
    editorLayout: {
      ...alienEditorLayout,
      fontFamily: '"Lucida Console", "Courier New", monospace',
      fontSize: 'clamp(0.98rem, 1.12vw, 1.24rem)',
      lineHeight: '1.38',
      letterSpacing: '0.032em',
      caretColor: '#ffd176',
      selectionColor: 'rgba(255, 203, 122, 0.16)',
      paddingTop: '0.08rem',
      paddingRight: '0',
      paddingBottom: '0',
      paddingLeft: '0',
      gutter: {
        lineCount: 24,
        width: '3rem',
        gap: '0.48rem',
        color: '#cf8d46',
        activeColor: '#ffd58a',
        fontSize: 'clamp(0.86rem, 1vw, 1.1rem)',
        formatter: (lineNumber: number) =>
          lineNumber === 1
            ? `${lineNumber.toString().padStart(2, ' ')}:`
            : `${lineNumber.toString().padStart(2, ' ')}: ~`,
      },
    },
  },
  {
    id: 'pip-os-word-processor',
    label: 'Pip-OS Word Processor',
    screenLayout: {
      scale: '0.94',
      safeInset: '1.4% 1.7% 2.4%',
    },
    screenFx: {
      ...amberScreenFxBase,
      screenTint:
        'radial-gradient(circle at 50% 22%, rgba(255, 180, 85, 0.12), rgba(72, 30, 8, 0.24) 40%, rgba(18, 8, 3, 0.98) 100%)',
      contentPaddingTop: '0.68rem',
      contentPaddingRight: '0.74rem',
      contentPaddingBottom: '0.72rem',
      contentPaddingLeft: '0.74rem',
    },
    chrome: {
      variant: 'banner-footer',
      textColor: '#ffba61',
      accentColor: '#ffd989',
      dimTextColor: '#d68f45',
      dividerColor: 'rgba(255, 200, 126, 0.82)',
      headerLines: [
        { text: 'PIP-OS WORD PROCESSOR [V4.1] <<<', align: 'left' },
        {
          text: 'FILE: [UNTITLED] | STATUS: READY | [CAPS LOCK]',
          align: 'left',
        },
        { text: 'OCT 23, 2077 | 09:12 AM', align: 'left' },
      ],
      footerLines: [
        {
          text: '^S: SAVE | ^O: OPEN | ^P: PRINT | ^H: HELP | [REC: 0 / 1024]',
          align: 'center',
        },
      ],
      dividerAfterHeader: true,
    },
    editorLayout: {
      ...alienEditorLayout,
      fontFamily: '"Lucida Console", "Courier New", monospace',
      fontSize: 'clamp(0.98rem, 1.12vw, 1.24rem)',
      lineHeight: '1.38',
      letterSpacing: '0.034em',
      caretColor: '#ffd278',
      selectionColor: 'rgba(255, 203, 122, 0.16)',
      paddingTop: '0.2rem',
      paddingRight: '0',
      paddingBottom: '0',
      paddingLeft: '0',
    },
  },
  {
    id: 'robco-unified-os',
    label: 'RobCo Unified Operating System',
    screenLayout: {
      scale: '0.94',
      safeInset: '1.35% 1.7% 2.45%',
    },
    screenFx: {
      ...amberScreenFxBase,
      screenBackground: '#100602',
      contentPaddingTop: '0.62rem',
      contentPaddingRight: '0.7rem',
      contentPaddingBottom: '0.72rem',
      contentPaddingLeft: '0.7rem',
    },
    chrome: {
      variant: 'banner-footer',
      textColor: '#ffbf66',
      accentColor: '#ffd88a',
      dimTextColor: '#d99245',
      dividerColor: 'rgba(255, 197, 122, 0.84)',
      headerLines: [
        { text: 'ROBCO UNIFIED OPERATING SYSTEM', align: 'left' },
        { text: 'WORD PROCESSOR V2.1 (P/N 53401)', align: 'left' },
        { text: 'DOCUMENT: UNTITLED        [INSERT MODE]        [CAPS] [NUM]', align: 'left' },
        { text: 'DATE: OCT 23, 2077', align: 'left' },
      ],
      footerLines: [
        { text: 'PAGE: 1   LINE: 1   COL: 1   [EDIT]', align: 'left' },
        {
          text: 'F1: SAVE   F2: LOAD   F3: NEW   F4: DELETE   F5: PRINT   F8: EXIT',
          align: 'left',
        },
      ],
      editorBordered: true,
    },
    editorLayout: {
      ...alienEditorLayout,
      fontFamily: '"Lucida Console", "Courier New", monospace',
      fontSize: 'clamp(0.98rem, 1.12vw, 1.22rem)',
      lineHeight: '1.36',
      letterSpacing: '0.034em',
      caretColor: '#ffd176',
      selectionColor: 'rgba(255, 201, 121, 0.16)',
      paddingTop: '0',
      paddingRight: '0',
      paddingBottom: '0',
      paddingLeft: '0',
    },
  },
  {
    id: 'vault-tec-data-terminal',
    label: 'Vault-Tec Data Terminal',
    screenLayout: {
      scale: '0.94',
      safeInset: '0.3% 0.45% 0.9%',
    },
    screenFx: {
      ...amberScreenFxBase,
      screenBackground: '#0f0602',
      contentPaddingTop: '0.24rem',
      contentPaddingRight: '0.28rem',
      contentPaddingBottom: '0.3rem',
      contentPaddingLeft: '0.28rem',
    },
    chrome: {
      variant: 'banner-footer',
      textColor: '#ffbc63',
      accentColor: '#ffd88b',
      dimTextColor: '#d79145',
      dividerColor: 'rgba(255, 199, 124, 0.82)',
      headerLines: [
        { text: 'VAULT-TEC DATA TERMINAL', align: 'left' },
        { text: 'SYSTEMS / DATA / OS V1.2', align: 'left' },
        { text: 'USER: OVERSEER // STATION 4', align: 'left' },
        {
          text: '[DOCUMENT: (EMPTY)]  PAGE: 1/1  MODE: EDIT  LN: 1  COL: 1  [SCAN] [SAVE] [LOAD]',
          align: 'left',
        },
      ],
      footerLines: [],
      dividerAfterHeader: true,
    },
    editorLayout: {
      ...alienEditorLayout,
      fontFamily: '"Lucida Console", "Courier New", monospace',
      fontSize: 'clamp(0.9rem, 0.98vw, 1.04rem)',
      lineHeight: '1.08',
      letterSpacing: '0.02em',
      caretColor: '#ffd27b',
      selectionColor: 'rgba(255, 204, 128, 0.14)',
      paddingTop: '0.02rem',
      paddingRight: '0',
      paddingBottom: '0',
      paddingLeft: '0',
      gutter: {
        lineCount: 24,
        width: '2.25rem',
        gap: '0.34rem',
        color: '#d58d41',
        activeColor: '#ffd98d',
        fontSize: 'clamp(0.8rem, 0.86vw, 0.92rem)',
        formatter: (lineNumber: number) => lineNumber.toString().padStart(2, '0'),
      },
    },
  },
  {
    id: 'robco-script-o-matic',
    label: 'RobCo Script-O-Matic',
    screenLayout: {
      scale: '0.88',
      safeInset: '1.1% 1.4% 2.1%',
    },
    screenFx: {
      ...robcoGreenScreenFxBase,
      contentPaddingTop: '0.48rem',
      contentPaddingRight: '0.56rem',
      contentPaddingBottom: '0.58rem',
      contentPaddingLeft: '0.56rem',
    },
    chrome: {
      variant: 'banner-footer',
      textColor: '#ffb85c',
      accentColor: '#ffd682',
      dimTextColor: '#d28c42',
      dividerColor: 'rgba(255, 195, 122, 0.82)',
      headerLines: [
        {
          text: '[ ROBCO ]    ROBCO INDUSTRIES - UNIFIED OPERATING SYSTEM (V4.82)',
          align: 'center',
        },
        { text: 'SCRIPT-O-MATIC V1.1', align: 'center' },
        {
          text: '16:34:21 // 2277.10.23 // MEM: 614K FREE // [NEW] [OPEN] [SAVE] [PRINT] [EXIT]',
          align: 'left',
        },
      ],
      footerLines: [
        {
          text: '[ DOC: UNTITLED.TXT ] [ LN: 1, COL: 1 ] [ INS ] [ CAP ] [ NUM ] [ MEM OK ]',
          align: 'left',
        },
      ],
      editorBordered: true,
    },
    editorLayout: {
      ...alienEditorLayout,
      fontFamily: '"Lucida Console", "Courier New", monospace',
      fontSize: 'clamp(0.88rem, 0.96vw, 1.02rem)',
      lineHeight: '1.24',
      letterSpacing: '0.03em',
      caretColor: '#ffd074',
      selectionColor: 'rgba(255, 197, 117, 0.16)',
      paddingTop: '0',
      paddingRight: '0',
      paddingBottom: '0',
      paddingLeft: '0',
    },
  },
];

export const THEMES_BY_ID = Object.fromEntries(
  THEMES.map((theme) => [theme.id, theme]),
) as Record<ThemeId, ThemeDefinition>;

export function isThemeId(value: string | null): value is ThemeId {
  return value !== null && value in THEMES_BY_ID;
}

export function getNextThemeId(currentThemeId: ThemeId): ThemeId {
  const currentIndex = THEME_ORDER.indexOf(currentThemeId);
  const nextIndex =
    currentIndex === -1 ? 0 : (currentIndex + 1) % THEME_ORDER.length;
  return THEME_ORDER[nextIndex];
}

export type RetroFocusThemeId = ThemeId;
export type RetroFocusThemeVariant = ThemeVariant;
export type RetroFocusThemeDefinition = ThemeDefinition;

export const DEFAULT_RETRO_FOCUS_THEME_ID = DEFAULT_THEME_ID;
export const RETRO_FOCUS_THEME_ORDER = THEME_ORDER;
export const RETRO_FOCUS_THEMES = THEMES;
export const RETRO_FOCUS_THEMES_BY_ID = THEMES_BY_ID;

export function isRetroFocusThemeId(
  value: string | null,
): value is RetroFocusThemeId {
  return isThemeId(value);
}

export function getNextRetroFocusThemeId(
  currentThemeId: RetroFocusThemeId,
): RetroFocusThemeId {
  return getNextThemeId(currentThemeId);
}
