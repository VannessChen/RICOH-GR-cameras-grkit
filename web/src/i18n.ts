export const LANGUAGES = [
  { code: 'en', label: 'English' },
  { code: 'zh-Hans', label: '简体中文' },
  { code: 'zh-Hant', label: '繁體中文' },
] as const;

export type Language = (typeof LANGUAGES)[number]['code'];

/** Text with inline code: a part that starts with a backtick is code. */
export type RichText = readonly string[];

const en = {
  language: 'Language',
  tagline:
    'Choose your camera and the effects you like. Then download one package and copy it to your SD card.',
  camera: 'Camera',
  firmware: 'Firmware',
  supported: 'Supported',
  planned: 'Planned',
  experimental: 'Experimental',
  experimentalNote: (name: string) =>
    `GRKit for ${name} is experimental. It has not been tested on a camera.`,
  effects: 'Effects',
  modules: 'Modules',
  /** Module names as the camera menu shows them in this language. */
  moduleNames: {
    imageControl: 'Image Control',
    framing: 'Framing',
    guides: 'Guides',
    dateStamp: 'Date Stamp',
    grain: 'Grain',
  },
  package: 'Package',
  install: 'Install',
  previewing: 'Previewing',
  samples: 'Sample photos',
  samplePhoto: 'GR III sample',
  sampleCredit: 'Sample: raw.pixls.us · CC0',
  decodingPhoto: 'Developing RAW photo…',
  upload: 'Open a DNG photo',
  yourPhoto: 'Your photo',
  photoError:
    'This photo could not be developed. Use an original GR III or GR IIIx DNG up to 64 MiB, then try again.',
  previewError: 'The preview cannot be shown.',
  retryPreview: 'Retry preview',
  uploadNote: 'GR III / GR IIIx DNG only, up to 64 MiB. Your photo stays in this browser.',
  previewNote:
    'The preview uses the GR III model and the recorded white balance. Color and exposure can differ from camera photos.',
  compare: 'Compare',
  original: 'Base development',
  preview: 'Preview',
  previewOf: (name: string) => `Preview ${name}`,
  /** The text before and after the selected count. */
  count: (max: number): readonly [string, string] => ['', ` of ${max} selected`],
  selectAll: 'Select all',
  clear: 'Clear',
  include: 'Include',
  icon: 'Icon',
  effect: 'Effect',
  look: 'Target look',
  type: 'Type',
  color: 'Color',
  mono: 'Mono',
  effectCount: 'Effects',
  loading: 'Loading effects…',
  loadError: 'The effects cannot be loaded. Reload the page to try again.',
  firmwareNote: 'Before you download, check the firmware version in the camera menu.',
  download: 'Download GRKit.zip',
  building: 'Preparing GRKit.zip…',
  ready: 'GRKit.zip is ready. Copy the contents of SD_ROOT to the card root.',
  buildError: 'GRKit.zip cannot be prepared. Reload the page to try again.',
  footer:
    'Supports RICOH GR III firmware 2.10 and GR IIIx firmware 1.60. GR IV and GR IV HDF firmware 1.11 support is experimental and has not been tested on a camera.',
  trademarks:
    'GRKit is not affiliated with or endorsed by Ricoh, Kodak, or Fujifilm. Some camera, film, and paper names are trademarks of their owners.',
  licenses: 'Effect licenses',
  thirdParty: 'Third-party licenses',
  recovery: 'Recovery guide',
  source: 'Source',
  modulesNote: 'The camera menu shows only the modules that you select.',
  effectsOff: 'Image Control is off. The package has no effects.',
  effectsUnavailable: (name: string) => `GRKit has no Image Control for ${name} yet.`,
  imageControlEmpty: 'Select at least one effect. Without effects, the camera hides Image Control.',
  noModules: 'Select at least one module.',
  modulesText: {
    imageControl: 'The effects below, with live-view preview.',
    framing:
      'XPan 65:24, 16:9, or 4:3 photos. The viewfinder stays 3:2 and dims the area outside the photo.',
    guides: 'Thirds, center, golden ratio, or diagonal lines inside the Framing area.',
    dateStamp: 'The date on photos in orange seven-segment digits.',
    grain: "Film grain from the camera's own grain stage, in three strengths. Photos only.",
  },
  framingXpanOnly:
    'XPan 65:24 photos. The viewfinder stays 3:2 and dims the area outside the photo.',
  steps: [
    ['Turn the camera off.'],
    ['Unzip GRKit.zip. Copy the contents of ', '`SD_ROOT', ' to the card root.'],
    [
      'Insert the card. If SD scripts are not enabled, hold MENU while you turn the camera on. Wait until the camera turns itself off.',
    ],
    ['Turn the camera on normally. Open ADJ and select the sixth entry, GRK.'],
    [
      'Optional: after the installation, remove ',
      '`{modeset}',
      ' and ',
      '`DEVELOP.MOD',
      ' from the card root.',
    ],
  ] as readonly RichText[],
  looks: {
    'portra-400-endura-v1': 'Kodak Portra 400 on Portra Endura paper',
    'gold-200-premier-v1': 'Kodak Gold 200 on Endura Premier paper',
    'ektar-100-edge-v1': 'Kodak Ektar 100 on Ektacolor Edge paper',
    'pro-400h-crystal-v1': 'Fujifilm Pro 400H on Crystal Archive paper',
    'xtra-400-crystal-v1': 'Fujifilm Superia X-TRA 400 on Crystal Archive paper',
    'vision3-250d-2383-v1': 'Kodak Vision3 250D 5207 on Vision 2383 print film',
    'vision3-500t-2393-v1': 'Kodak Vision3 500T 5219 on Vision Premier 2393 print film',
    'portra-800-push2-v1': 'Kodak Portra 800 pushed two stops to EI 3200, on Portra Endura paper',
    'kodachrome-64-v1': 'Kodak Kodachrome 64 slide film, scanned',
    'velvia-100-v1': 'Fujifilm Velvia 100 slide film, scanned',
    'portra-160-ultra-v1': 'Kodak Portra 160 on Ultra Endura paper',
    'trix-400-mono-v1': 'Kodak Tri-X 400 on Polymax Fine-Art paper',
    'red-filter-mono-v1': 'Kodak Tri-X 400 behind a Wratten 25 red filter',
    'fp-100c-v1': 'Fujifilm FP-100C instant film',
    'aerocolor-iv-v1': 'Kodak Aerocolor IV aerial negative on Portra Endura paper',
    'exr-200t-2383-v1': 'Kodak EXR 200T on Vision 2383 print film',
  } as Readonly<Record<string, string>>,
};

export type Strings = typeof en;

const zhHans: Strings = {
  language: '语言',
  tagline: '选择你的相机和喜欢的效果，下载安装包，然后复制到存储卡。',
  camera: '相机',
  firmware: '固件',
  supported: '已支持',
  planned: '计划支持',
  experimental: '实验性',
  experimentalNote: (name) => `${name} 的 GRKit 是实验性的，未经真机测试。`,
  effects: '效果',
  modules: '模块',
  /** Module names as the camera menu shows them in this language. */
  moduleNames: {
    imageControl: '影像控制',
    framing: '画幅',
    guides: '辅助线',
    dateStamp: '加印日期',
    grain: '颗粒感',
  },
  package: '安装包',
  install: '安装',
  previewing: '正在预览',
  samples: '示例照片',
  samplePhoto: 'GR III 样片',
  sampleCredit: '样片：raw.pixls.us · CC0',
  decodingPhoto: '正在显影 RAW 照片…',
  upload: '打开 DNG 照片',
  yourPhoto: '你的照片',
  photoError: '无法显影这张照片。请使用不超过 64 MiB 的 GR III 或 GR IIIx 原始 DNG，然后重试。',
  previewError: '无法显示预览。',
  retryPreview: '重试预览',
  uploadNote: '仅接受 GR III／GR IIIx 的 DNG，最大 64 MiB。照片只在本浏览器中处理。',
  previewNote: '预览使用 GR III 模型和拍摄时的白平衡。颜色和曝光可能与机内成片不同。',
  compare: '对比',
  original: '基础显影',
  preview: '预览',
  previewOf: (name) => `预览 ${name}`,
  count: (max) => ['已选 ', ` / ${max}`],
  selectAll: '全选',
  clear: '清空',
  include: '包含',
  icon: '图标',
  effect: '效果',
  look: '模拟胶片',
  type: '类型',
  color: '彩色',
  mono: '黑白',
  effectCount: '效果数量',
  loading: '正在加载效果…',
  loadError: '无法加载效果。请刷新页面后重试。',
  firmwareNote: '下载前，请在相机菜单中确认固件版本。',
  download: '下载 GRKit.zip',
  building: '正在准备 GRKit.zip…',
  ready: 'GRKit.zip 已就绪。请把 SD_ROOT 中的内容复制到存储卡根目录。',
  buildError: '无法准备 GRKit.zip。请刷新页面后重试。',
  footer:
    '目前支持 RICOH GR III 固件 2.10 和 GR IIIx 固件 1.60。GR IV 与 GR IV HDF 固件 1.11 为实验性支持，未经真机测试。',
  trademarks:
    'GRKit 与 Ricoh、Kodak、Fujifilm 无关，也未获得它们的认可。部分相机、胶片和相纸名称是其所有者的商标。',
  licenses: '效果许可',
  thirdParty: '第三方许可',
  recovery: '恢复指南',
  source: '源代码',
  modulesNote: '相机菜单只显示你选择的模块。',
  effectsOff: '影像控制已关闭，安装包中不包含效果。',
  effectsUnavailable: (name) => `GRKit 暂不支持 ${name} 的影像控制。`,
  imageControlEmpty: '请至少选择一个效果。没有效果时，相机会隐藏影像控制。',
  noModules: '请至少选择一个模块。',
  modulesText: {
    imageControl: '使用下方选择的效果，可在实时显示中预览。',
    framing: '以 XPan 65:24、16:9 或 4:3 的长宽比拍摄。显示屏仍按 3:2 显示，照片以外的区域会变暗。',
    guides: '在画幅范围内显示辅助线：三分法、中心、黄金分割或对角线。',
    dateStamp: '在照片右下角加印橙色七段数码日期。',
    grain: '使用相机内置的颗粒感处理，分弱、中、强三档。不影响影片。',
  },
  framingXpanOnly: '以 XPan 65:24 的长宽比拍摄。显示屏仍按 3:2 显示，照片以外的区域会变暗。',
  steps: [
    ['关闭相机。'],
    ['解压 GRKit.zip。把 ', '`SD_ROOT', ' 中的内容复制到存储卡根目录。'],
    ['插入存储卡。如果尚未启用 SD 脚本，请按住 MENU 按钮开机，然后等待相机自动关机。'],
    ['正常开机。按下 ADJ 杆进入 ADJ 模式，选择第 6 个功能 GRK。'],
    ['可选：安装完成后，从存储卡根目录删除 ', '`{modeset}', ' 和 ', '`DEVELOP.MOD', '。'],
  ],
  looks: {
    'portra-400-endura-v1': 'Kodak Portra 400，Portra Endura 相纸',
    'gold-200-premier-v1': 'Kodak Gold 200，Endura Premier 相纸',
    'ektar-100-edge-v1': 'Kodak Ektar 100，Ektacolor Edge 相纸',
    'pro-400h-crystal-v1': 'Fujifilm Pro 400H，Crystal Archive 相纸',
    'xtra-400-crystal-v1': 'Fujifilm Superia X-TRA 400，Crystal Archive 相纸',
    'vision3-250d-2383-v1': 'Kodak Vision3 250D 5207，Vision 2383 拷贝片',
    'vision3-500t-2393-v1': 'Kodak Vision3 500T 5219，Vision Premier 2393 拷贝片',
    'portra-800-push2-v1': 'Kodak Portra 800 增感两档至 EI 3200，Portra Endura 相纸',
    'kodachrome-64-v1': 'Kodak Kodachrome 64 正片，直接扫描',
    'velvia-100-v1': 'Fujifilm Velvia 100 正片，直接扫描',
    'portra-160-ultra-v1': 'Kodak Portra 160，Ultra Endura 相纸',
    'trix-400-mono-v1': 'Kodak Tri-X 400，Polymax Fine-Art 相纸',
    'red-filter-mono-v1': 'Kodak Tri-X 400 加 Wratten 25 红滤镜',
    'fp-100c-v1': 'Fujifilm FP-100C 即显胶片',
    'aerocolor-iv-v1': 'Kodak Aerocolor IV 航拍负片，Portra Endura 相纸',
    'exr-200t-2383-v1': 'Kodak EXR 200T，Vision 2383 拷贝片',
  },
};

const zhHant: Strings = {
  language: '語言',
  tagline: '選擇你的相機和喜歡的效果，下載安裝包，然後複製到記憶卡。',
  camera: '相機',
  firmware: '韌體',
  supported: '已支援',
  planned: '規劃中',
  experimental: '實驗性',
  experimentalNote: (name) => `${name} 的 GRKit 為實驗性支援，未經實機測試。`,
  effects: '效果',
  modules: '模組',
  /** Module names as the camera menu shows them in this language. */
  moduleNames: {
    imageControl: '影像控制',
    framing: '畫幅',
    guides: '輔助線',
    dateStamp: '加印日期',
    grain: '粒紋感',
  },
  package: '安裝包',
  install: '安裝',
  previewing: '正在預覽',
  samples: '範例照片',
  samplePhoto: 'GR III 範例照片',
  sampleCredit: '範例照片：raw.pixls.us · CC0',
  decodingPhoto: '正在顯影 RAW 照片…',
  upload: '開啟 DNG 照片',
  yourPhoto: '你的照片',
  photoError: '無法顯影這張照片。請使用不超過 64 MiB 的 GR III 或 GR IIIx 原始 DNG，然後重試。',
  previewError: '無法顯示預覽。',
  retryPreview: '重試預覽',
  uploadNote: '僅接受 GR III／GR IIIx 的 DNG，最大 64 MiB。照片只在此瀏覽器中處理。',
  previewNote: '預覽使用 GR III 模型和拍攝時的白平衡。顏色與曝光可能與機內成片不同。',
  compare: '對比',
  original: '基礎顯影',
  preview: '預覽',
  previewOf: (name) => `預覽 ${name}`,
  count: (max) => ['已選 ', ` / ${max}`],
  selectAll: '全選',
  clear: '清除',
  include: '包含',
  icon: '圖示',
  effect: '效果',
  look: '模擬底片',
  type: '類型',
  color: '彩色',
  mono: '黑白',
  effectCount: '效果數量',
  loading: '正在載入效果…',
  loadError: '無法載入效果。請重新整理頁面後再試。',
  firmwareNote: '下載前，請在相機功能表中確認韌體版本。',
  download: '下載 GRKit.zip',
  building: '正在準備 GRKit.zip…',
  ready: 'GRKit.zip 已就緒。請把 SD_ROOT 中的內容複製到記憶卡根目錄。',
  buildError: '無法準備 GRKit.zip。請重新整理頁面後再試。',
  footer:
    '目前支援 RICOH GR III 韌體 2.10 與 GR IIIx 韌體 1.60。GR IV 與 GR IV HDF 韌體 1.11 為實驗性支援，未經實機測試。',
  trademarks:
    'GRKit 與 Ricoh、Kodak、Fujifilm 無關，也未獲得其認可。部分相機、底片與相紙名稱為其所有者的商標。',
  licenses: '效果授權',
  thirdParty: '第三方授權',
  recovery: '復原指南',
  source: '原始碼',
  modulesNote: '相機功能表只顯示你選擇的模組。',
  effectsOff: '影像控制已關閉，安裝包中不包含效果。',
  effectsUnavailable: (name) => `GRKit 暫不支援 ${name} 的影像控制。`,
  imageControlEmpty: '請至少選擇一個效果。沒有效果時，相機會隱藏影像控制。',
  noModules: '請至少選擇一個模組。',
  modulesText: {
    imageControl: '使用下方選擇的效果，可在實時顯示中預覽。',
    framing: '以 XPan 65:24、16:9 或 4:3 的長寬比拍攝。顯示屏仍按 3:2 顯示，照片以外的區域會變暗。',
    guides: '在畫幅範圍內顯示輔助線：三分法、中心、黃金分割或對角線。',
    dateStamp: '在照片右下角加印橙色七段數字日期。',
    grain: '使用相機內建的粒紋感處理，分弱、中、強三檔。不影響影片。',
  },
  framingXpanOnly: '以 XPan 65:24 的長寬比拍攝。顯示屏仍按 3:2 顯示，照片以外的區域會變暗。',
  steps: [
    ['關閉相機。'],
    ['解壓縮 GRKit.zip。把 ', '`SD_ROOT', ' 中的內容複製到記憶卡根目錄。'],
    ['插入記憶卡。如果尚未啟用 SD 腳本，請按住 MENU 按鈕開機，然後等待相機自動關機。'],
    ['正常開機。按下 ADJ 桿進入 ADJ 模式，選擇第 6 個功能 GRK。'],
    ['選用：安裝完成後，從記憶卡根目錄刪除 ', '`{modeset}', ' 和 ', '`DEVELOP.MOD', '。'],
  ],
  looks: {
    'portra-400-endura-v1': 'Kodak Portra 400，Portra Endura 相紙',
    'gold-200-premier-v1': 'Kodak Gold 200，Endura Premier 相紙',
    'ektar-100-edge-v1': 'Kodak Ektar 100，Ektacolor Edge 相紙',
    'pro-400h-crystal-v1': 'Fujifilm Pro 400H，Crystal Archive 相紙',
    'xtra-400-crystal-v1': 'Fujifilm Superia X-TRA 400，Crystal Archive 相紙',
    'vision3-250d-2383-v1': 'Kodak Vision3 250D 5207，Vision 2383 拷貝片',
    'vision3-500t-2393-v1': 'Kodak Vision3 500T 5219，Vision Premier 2393 拷貝片',
    'portra-800-push2-v1': 'Kodak Portra 800 增感兩檔至 EI 3200，Portra Endura 相紙',
    'kodachrome-64-v1': 'Kodak Kodachrome 64 正片，直接掃描',
    'velvia-100-v1': 'Fujifilm Velvia 100 正片，直接掃描',
    'portra-160-ultra-v1': 'Kodak Portra 160，Ultra Endura 相紙',
    'trix-400-mono-v1': 'Kodak Tri-X 400，Polymax Fine-Art 相紙',
    'red-filter-mono-v1': 'Kodak Tri-X 400 加 Wratten 25 紅濾鏡',
    'fp-100c-v1': 'Fujifilm FP-100C 即顯底片',
    'aerocolor-iv-v1': 'Kodak Aerocolor IV 航拍負片，Portra Endura 相紙',
    'exr-200t-2383-v1': 'Kodak EXR 200T，Vision 2383 拷貝片',
  },
};

export const STRINGS: Readonly<Record<Language, Strings>> = {
  en,
  'zh-Hans': zhHans,
  'zh-Hant': zhHant,
};

const STORAGE_KEY = 'grkit.language';

function isLanguage(value: unknown): value is Language {
  return LANGUAGES.some((language) => language.code === value);
}

/** Maps browser language tags to a supported language. Traditional script wins for TW, HK, and MO. */
export function matchLanguage(tags: readonly string[]): Language {
  for (const tag of tags) {
    const lower = tag.toLowerCase();
    if (lower.startsWith('zh')) {
      return /hant|-(tw|hk|mo)\b/.test(lower) ? 'zh-Hant' : 'zh-Hans';
    }
    if (lower.startsWith('en')) {
      return 'en';
    }
  }
  return 'en';
}

export function initialLanguage(): Language {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (isLanguage(saved)) {
      return saved;
    }
  } catch {
    // Storage can be unavailable, for example in a private window.
  }
  return matchLanguage(navigator.languages ?? [navigator.language]);
}

/** Applies a language to the document and remembers it for the next visit. */
export function applyLanguage(language: Language): void {
  document.documentElement.lang = language;
  try {
    localStorage.setItem(STORAGE_KEY, language);
  } catch {
    // The choice then lasts only for this page view.
  }
}
