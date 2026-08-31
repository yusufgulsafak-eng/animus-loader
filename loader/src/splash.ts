const splash = document.querySelector<HTMLElement>("#animus-startup-splash");
const app = document.querySelector<HTMLElement>("#app");

const startedAt = performance.now();
const minimumVisibleMs = 1500;
const maximumVisibleMs = 12000;
const fadeMs = 460;

let dismissStarted = false;
let observer: MutationObserver | null = null;

function appHasRenderedContent(): boolean {
  if (!app) return false;

  if (app.childElementCount > 0) {
    return true;
  }

  return Boolean(app.textContent?.trim());
}

function removeSplash(): void {
  if (!splash || dismissStarted) return;

  dismissStarted = true;
  observer?.disconnect();
  observer = null;

  const elapsed = performance.now() - startedAt;
  const remaining = Math.max(0, minimumVisibleMs - elapsed);

  window.setTimeout(() => {
    splash.classList.add("is-leaving");
    document.body.classList.remove("splash-active");

    window.setTimeout(() => {
      splash.remove();
    }, fadeMs);
  }, remaining);
}

if (!splash) {
  document.body.classList.remove("splash-active");
} else if (!app) {
  removeSplash();
} else {
  observer = new MutationObserver(() => {
    if (appHasRenderedContent()) {
      removeSplash();
    }
  });

  observer.observe(app, {
    childList: true,
    subtree: true,
    characterData: true,
  });

  if (appHasRenderedContent()) {
    removeSplash();
  }

  window.setTimeout(removeSplash, maximumVisibleMs);
}
