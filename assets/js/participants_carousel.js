(() => {
  function initCarousel(strip) {
    const groupSize = Math.max(
      1,
      parseInt(strip.getAttribute("data-carousel-group") || "10", 10) || 10
    );
    const intervalMs = Math.max(
      1500,
      parseInt(strip.getAttribute("data-carousel-interval-ms") || "5000", 10) ||
        5000
    );

    const prefersReducedMotion =
      window.matchMedia &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches;

    const tiles = Array.from(strip.children).filter(
      (el) => el && el.nodeType === Node.ELEMENT_NODE
    );
    const n = tiles.length;
    if (n <= groupSize || prefersReducedMotion) return;

    const fixed = tiles.filter((el) =>
      el.classList.contains("activity-participants-fixed")
    );
    const rotatable = tiles.filter(
      (el) => !el.classList.contains("activity-participants-fixed")
    );
    const visibleRotatableCount = Math.max(1, groupSize - fixed.length);
    if (rotatable.length <= visibleRotatableCount) return;

    strip.classList.add("activity-participants-carousel");

    let group = 0;
    const groups = Math.ceil(rotatable.length / visibleRotatableCount);

    function visibleIndexSet() {
      const set = new Set();
      for (const el of fixed) {
        set.add(tiles.indexOf(el));
      }
      const base = group * visibleRotatableCount;
      for (let i = 0; i < visibleRotatableCount; i++) {
        const rot = rotatable[(base + i) % rotatable.length];
        set.add(tiles.indexOf(rot));
      }
      return set;
    }

    function applyVisible() {
      const visible = visibleIndexSet();
      for (let i = 0; i < n; i++) {
        tiles[i].classList.toggle("hidden", !visible.has(i));
      }
    }

    function next() {
      strip.classList.add("activity-participants-fadeout");
      window.setTimeout(() => {
        group = (group + 1) % groups;
        applyVisible();
        strip.classList.remove("activity-participants-fadeout");
      }, 260);
    }

    applyVisible();
    window.setInterval(next, intervalMs);
  }

  function boot() {
    document
      .querySelectorAll(
        '.activity-participants-strip[data-participants-carousel="1"]'
      )
      .forEach(initCarousel);
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot, { once: true });
  } else {
    boot();
  }
})();
