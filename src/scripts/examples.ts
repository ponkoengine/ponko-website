import { animate, hover, inView } from "motion";
const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
const examples = [...document.querySelectorAll<HTMLElement>("[data-example]")];
const tabs = [...document.querySelectorAll<HTMLButtonElement>("[data-game]")];
let active = examples[0];
let iframe: HTMLIFrameElement | null = null;
let startupTimer: ReturnType<typeof setTimeout> | undefined;
function visiblePane(pane: Element): boolean {
  const r = pane.getBoundingClientRect();
  const area =
    Math.max(0, Math.min(r.right, innerWidth) - Math.max(0, r.left)) *
    Math.max(0, Math.min(r.bottom, innerHeight) - Math.max(0, r.top));
  return r.width * r.height > 0 && area / (r.width * r.height) >= 0.15;
}
function sendVisibility(frame: HTMLIFrameElement) {
  const pane = frame.closest(".play-pane")!;
  const visible = visiblePane(pane);
  frame.contentWindow?.postMessage(
    { type: "ponko-visibility", visible },
    location.origin,
  );
  return visible;
}
const launches = new Map(
  examples.map((example) => [
    example,
    example.querySelector(".launch-screen")!.cloneNode(true),
  ]),
);
function status(example: HTMLElement, text: string) {
  example.querySelector("[data-game-status]")!.textContent = text;
}
function stop() {
  clearTimeout(startupTimer);
  if (!iframe) return;
  const example = iframe.closest<HTMLElement>("[data-example]")!;
  iframe.remove();
  iframe = null;
  example
    .querySelector("[data-stage]")!
    .replaceChildren(launches.get(example)!.cloneNode(true));
  example.querySelector("[data-stop]")!.setAttribute("hidden", "");
  example.querySelector(".play-pane")!.classList.remove("running");
  status(example, "Stopped");
}
function failure(message: string) {
  if (!iframe) return;
  stop();
  const box = document.createElement("div");
  box.className = "error-state";
  const text = document.createElement("p");
  text.textContent = message;
  const retry = document.createElement("button");
  retry.type = "button";
  retry.className = "launch";
  retry.dataset.launch = "";
  retry.textContent = "Retry game";
  box.append(text, retry);
  active.querySelector("[data-stage]")!.replaceChildren(box);
  status(active, "Could not start");
}
function launch(example: HTMLElement) {
  stop();
  iframe = document.createElement("iframe");
  iframe.src = `/play/${example.dataset.example}/`;
  iframe.title = `${example.querySelector("h2")!.textContent} playable game`;
  iframe.allow = "fullscreen; autoplay";
  iframe.addEventListener("load", () => {
    if (iframe) sendVisibility(iframe);
  });
  example.querySelector("[data-stage]")!.replaceChildren(iframe);
  example.querySelector("[data-stop]")!.removeAttribute("hidden");
  status(example, "Loading WASM…");
  startupTimer = setTimeout(
    () =>
      failure(
        "The game took too long to start. Check your connection and browser graphics support, then retry.",
      ),
    50_000,
  );
}
function select(id: string) {
  const next = examples.find((e) => e.dataset.example === id)!;
  if (next === active) return;
  stop();
  active.hidden = true;
  next.hidden = false;
  active = next;
  tabs.forEach((tab) => {
    const selected = tab.dataset.game === id;
    tab.setAttribute("aria-selected", String(selected));
    tab.tabIndex = selected ? 0 : -1;
  });
  if (!reduced.matches)
    animate(next, { opacity: [0.5, 1], y: [5, 0] }, { duration: 0.2 });
}
tabs.forEach((tab, index) => {
  tab.addEventListener("click", () => select(tab.dataset.game!));
  tab.addEventListener("keydown", (e) => {
    let next = index;
    if (e.key === "ArrowRight") next = (index + 1) % tabs.length;
    else if (e.key === "ArrowLeft")
      next = (index - 1 + tabs.length) % tabs.length;
    else if (e.key === "Home") next = 0;
    else if (e.key === "End") next = tabs.length - 1;
    else return;
    e.preventDefault();
    tabs[next].focus();
    select(tabs[next].dataset.game!);
  });
});
examples.forEach((example) => {
  example.addEventListener("click", async (e) => {
    const button = (e.target as HTMLElement).closest<HTMLButtonElement>(
      "button",
    );
    if (!button) return;
    if (button.hasAttribute("data-launch")) launch(example);
    if (button.hasAttribute("data-stop")) stop();
    if (button.dataset.view) {
      const view = button.dataset.view;
      if (view === "code") stop();
      example.querySelector<HTMLElement>("[data-layout]")!.dataset.layout =
        view;
      example
        .querySelectorAll<HTMLButtonElement>("[data-view]")
        .forEach((b) => b.setAttribute("aria-pressed", String(b === button)));
      if (!reduced.matches)
        animate(
          example.querySelector<HTMLElement>(".workspace")!,
          { opacity: [0.65, 1] },
          { duration: 0.16 },
        );
    }
    if (button.hasAttribute("data-fullscreen")) {
      try {
        if (document.fullscreenElement) await document.exitFullscreen();
        else await example.querySelector(".play-pane")!.requestFullscreen();
        iframe?.focus();
      } catch {
        status(example, "Use “Open in a tab” for a larger view.");
      }
    }
  });
  example
    .querySelector<HTMLSelectElement>("[data-file]")!
    .addEventListener("change", (e) => {
      const select = e.target as HTMLSelectElement;
      example
        .querySelectorAll<HTMLElement>("[data-code-file]")
        .forEach(
          (code) => (code.hidden = code.dataset.codeFile !== select.value),
        );
      const source = select.selectedOptions[0]
        .textContent!.split(" · ")
        .at(-1)!;
      example.querySelector<HTMLAnchorElement>("[data-source-link]")!.href =
        `https://github.com/ponkoengine/ponko-website/blob/main/games/src/${source}`;
      example.querySelector(".code-scroll")!.scrollTop = 0;
    });
});
window.addEventListener("message", (e) => {
  if (e.origin !== location.origin || e.source !== iframe?.contentWindow)
    return;
  if (e.data?.type === "ponko-ready") {
    clearTimeout(startupTimer);
    status(active, "Running · P to pause");
    active.querySelector(".play-pane")!.classList.add("running");
    if (iframe && sendVisibility(iframe)) {
      iframe.focus();
      iframe.contentWindow?.postMessage(
        { type: "ponko-focus" },
        location.origin,
      );
    } else if (document.activeElement === iframe) {
      // winit may focus its canvas during startup. Restore parent focus
      // without scrolling an offscreen game back into view.
      tabs.find(tab => tab.dataset.game === active.dataset.example)?.focus({ preventScroll: true });
    }
  } else if (e.data?.type === "ponko-error") failure(e.data.message);
});
const visibility = new IntersectionObserver(
  (entries) => {
    for (const entry of entries) {
      const frame = entry.target.querySelector("iframe");
      const visible = entry.intersectionRatio >= 0.15;
      frame?.contentWindow?.postMessage(
        { type: "ponko-visibility", visible },
        location.origin,
      );
    }
  },
  { threshold: 0.15 },
);
examples.forEach((example) =>
  visibility.observe(example.querySelector(".play-pane")!),
);
window.addEventListener("pagehide", stop);
hover(".launch", (element) => {
  if (reduced.matches) return;
  animate(
    element,
    { scale: 1.025 },
    { type: "spring", stiffness: 500, damping: 30 },
  );
  return () => {
    animate(element, { scale: 1 }, { duration: 0.15 });
  };
});
inView(".about", (element) => {
  if (!reduced.matches)
    animate(element, { y: [10, 0], opacity: [0.7, 1] }, { duration: 0.35 });
});
const copy = document.querySelector<HTMLButtonElement>("#copy")!;
copy.addEventListener("click", async () => {
  try {
    await navigator.clipboard.writeText("cargo add ponko");
    copy.textContent = "Copied";
    document.querySelector("#copy-status")!.textContent =
      "Copied cargo add ponko";
  } catch {
    document.querySelector("#copy-status")!.textContent =
      "Select and copy the command.";
  }
});
