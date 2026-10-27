import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './App';
import { canWriteCards } from './card';
import { loadLastCard } from './cardStore';
import { loadImports } from './import/store';
import { loadFonts } from './fonts';
import { initialLanguage, showLanguage } from './i18n';
// The site serves its fonts itself, so a page view sends no request to a font host.
// loadFonts adds the Chinese or Japanese fonts when a page in that language needs them.
import '@fontsource/ibm-plex-sans/400.css';
import '@fontsource/ibm-plex-sans/500.css';
import '@fontsource/ibm-plex-sans/600.css';
import '@fontsource/ibm-plex-sans-condensed/500.css';
import '@fontsource/ibm-plex-sans-condensed/700.css';
import '@fontsource/ibm-plex-mono/400.css';
import '@fontsource/ibm-plex-mono/500.css';
import './styles.css';

const language = initialLanguage();
showLanguage(language);
loadFonts(language);

/** The result of `task`, or `fallback` when the store is slow, so it does not hold the page back. */
const soon = <T,>(task: Promise<T>, fallback: T) =>
  Promise.race([task, new Promise<T>((resolve) => setTimeout(() => resolve(fallback), 800))]);

// The effects imported and the card connected on an earlier visit.
const [imports, lastCard] = await Promise.all([
  soon(loadImports(), []),
  canWriteCards() ? soon(loadLastCard(), null) : null,
]);

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App initialImports={imports} initialCard={lastCard} />
  </StrictMode>,
);
