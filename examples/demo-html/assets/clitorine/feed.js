import { byId } from "./dom.js";
import {
  bindMedia,
  evict,
  groupHeight,
  protectedGroup,
  visibleAnchor,
} from "./feed-cards.js";
import {
  bindFeedNavigation,
  restoreReadingPosition,
} from "./feed-navigation.js";
import { bindFeedUpdates } from "./feed-updates.js";

function createFeedWindow() {
  const controller = byId("feed-controller");
  const frame = byId("feed-window");
  if (!controller || !frame) {
    return null;
  }
  const wall = byId("wall");
  const status = byId("feed-status");
  const olderLink = frame.querySelector('[data-feed-load="older"]');
  const newerLink = frame.querySelector('[data-feed-load="newer"]');
  const topSpacer = frame.querySelector('[data-feed-spacer="top"]');
  const bottomSpacer = frame.querySelector('[data-feed-spacer="bottom"]');
  const end = frame.querySelector("[data-feed-end]");
  const retry = document.createElement("button");
  retry.type = "button";
  retry.className = "btn btn-primary feed-retry";
  retry.textContent = olderLink.textContent;
  retry.hidden = true;
  status.after(retry);
  const pageSize = 20;
  const maxCards = 60;
  let failedDirection = null;
  let direction = "older";
  let raf = null;

  const feed = {
    controller,
    frame,
    wall,
    status,
    newerLink,
    groups: [Array.from(wall.querySelectorAll("article"))],
    ceiling: frame.dataset.ceiling,
    hasOlder: frame.dataset.hasOlder === "true",
    hasNewer: frame.dataset.hasNewer === "true",
    topHeight: 0,
    bottomHeight: 0,
    request: null,
    suspended: false,
    restoring: false,
    generation: 0,
    lastScroll: window.scrollY,
    anchor: null,
    leaving: false,
    observer: null,
    resizeObserver: null,
    cards: () => Array.from(wall.querySelectorAll("article")),
    visibleAnchor: () => visibleAnchor(wall),
    evict: (group) => evict(group, feed.resizeObserver),
  };

  feed.correctAnchor = function (saved) {
    if (!saved) {
      return;
    }
    const card = byId(saved.id);
    if (card) {
      const change = card.getBoundingClientRect().top - saved.offset;
      if (Math.abs(change) > 0.5) {
        window.scrollBy({ top: change, behavior: "instant" });
        feed.lastScroll = window.scrollY;
      }
    }
    feed.anchor = feed.visibleAnchor();
  };

  feed.setSpacers = function () {
    topSpacer.style.height = Math.max(0, feed.topHeight) + "px";
    bottomSpacer.style.height = Math.max(0, feed.bottomHeight) + "px";
  };

  feed.updateControls = function () {
    const mounted = feed.cards();
    const first = mounted[0];
    const last = mounted[mounted.length - 1];
    newerLink.hidden = !feed.hasNewer;
    olderLink.hidden = !feed.hasOlder;
    end.hidden = feed.hasOlder;
    newerLink.href =
      "/?after=" +
      (first ? first.id.slice(5) : "0") +
      "&ceiling=" +
      feed.ceiling;
    olderLink.href =
      "/?before=" +
      (last ? last.id.slice(5) : "0") +
      "&ceiling=" +
      feed.ceiling;
    frame.dataset.ceiling = feed.ceiling;
    frame.dataset.hasOlder = String(feed.hasOlder);
    frame.dataset.hasNewer = String(feed.hasNewer);
    frame.dataset.newest = first ? first.id.slice(5) : "0";
    frame.dataset.oldest = last ? last.id.slice(5) : "0";
    if (feed.observer) {
      feed.observer.disconnect();
      if (first) {
        feed.observer.observe(first);
      }
      if (last && last !== first) {
        feed.observer.observe(last);
      }
    }
  };

  feed.fetchWindow = async function (url, signal) {
    const response = await fetch(url, {
      headers: { "X-MyFeed-Ajax": "1" },
      credentials: "same-origin",
      signal,
    });
    if (!response.ok) {
      throw new Error("HTTP " + response.status);
    }
    const template = document.createElement("template");
    template.innerHTML = await response.text();
    const next = template.content.querySelector("#feed-window");
    if (
      !next ||
      next.querySelectorAll("article").length > pageSize ||
      !/^\d+$/.test(next.dataset.ceiling)
    ) {
      throw new Error("Invalid feed window");
    }
    return {
      frame: next,
      missing: response.headers.get("x-myfeed-missing") === "1",
    };
  };

  async function load(which, manual) {
    if (
      feed.request ||
      feed.restoring ||
      (!manual && feed.suspended) ||
      (which === "older" ? !feed.hasOlder : !feed.hasNewer)
    ) {
      return;
    }
    const mounted = feed.cards();
    const trim =
      mounted.length + pageSize > maxCards
        ? which === "older"
          ? feed.groups[0]
          : feed.groups[feed.groups.length - 1]
        : null;
    if (trim && protectedGroup(trim)) {
      status.textContent = controller.dataset.blocked;
      failedDirection = which;
      retry.hidden = false;
      feed.suspended = true;
      return;
    }
    feed.suspended = false;
    retry.hidden = true;
    feed.request = new AbortController();
    const activeRequest = feed.request;
    const activeGeneration = feed.generation;
    status.textContent = controller.dataset.loading;
    try {
      const result = await feed.fetchWindow(
        which === "older" ? olderLink.href : newerLink.href,
        activeRequest.signal,
      );
      if (activeGeneration !== feed.generation) {
        return;
      }
      const saved = feed.visibleAnchor();
      const existing = new Set(mounted.map((card) => card.id));
      const added = Array.from(result.frame.querySelectorAll("article")).filter(
        (card) => !existing.has(card.id),
      );
      if (trim && protectedGroup(trim)) {
        status.textContent = controller.dataset.blocked;
        feed.suspended = true;
        failedDirection = which;
        retry.hidden = false;
        return;
      }
      if (!added.length) {
        if (which === "older") {
          feed.hasOlder = false;
        } else {
          feed.hasNewer = false;
        }
      } else {
        if (trim) {
          const height = groupHeight(wall, trim);
          if (which === "older") {
            feed.groups.shift();
            feed.topHeight += height;
            feed.hasNewer = true;
          } else {
            feed.groups.pop();
            feed.bottomHeight += height;
            feed.hasOlder = true;
          }
          feed.evict(trim);
        }
        if (which === "older") {
          wall.append(...added);
          feed.groups.push(added);
          feed.bottomHeight = Math.max(
            0,
            feed.bottomHeight - groupHeight(wall, added),
          );
          feed.hasOlder = result.frame.dataset.hasOlder === "true";
          if (!feed.hasOlder) {
            feed.bottomHeight = 0;
          }
        } else {
          wall.prepend(...added);
          feed.groups.unshift(added);
          feed.topHeight = Math.max(
            0,
            feed.topHeight - groupHeight(wall, added),
          );
          feed.hasNewer = result.frame.dataset.hasNewer === "true";
          if (!feed.hasNewer) {
            feed.topHeight = 0;
          }
        }
        added.forEach(function (card) {
          if (feed.resizeObserver) {
            feed.resizeObserver.observe(card);
          }
        });
      }
      status.textContent = "";
      feed.setSpacers();
      feed.updateControls();
      feed.correctAnchor(saved);
      failedDirection = null;
    } catch (error) {
      if (error.name !== "AbortError") {
        console.error("MyFeed loading failed", error);
        status.textContent = controller.dataset.error;
        failedDirection = which;
        retry.hidden = false;
        feed.suspended = true;
      }
    } finally {
      if (feed.request === activeRequest) {
        feed.request = null;
      }
      feed.savePosition();
    }
  }

  feed.considerLoading = function () {
    if (
      !feed.observer ||
      feed.request ||
      feed.restoring ||
      feed.suspended ||
      document.hidden
    ) {
      return;
    }
    const mounted = feed.cards();
    if (!mounted.length) {
      return;
    }
    if (
      direction === "older" &&
      feed.hasOlder &&
      mounted[mounted.length - 1].getBoundingClientRect().top <
        window.innerHeight + 600
    ) {
      load("older", false);
    } else if (
      direction === "newer" &&
      feed.hasNewer &&
      mounted[0].getBoundingClientRect().bottom > -600
    ) {
      load("newer", false);
    }
  };

  feed.observer =
    typeof IntersectionObserver === "undefined"
      ? null
      : new IntersectionObserver(feed.considerLoading, {
          rootMargin: "600px 0px",
        });
  feed.resizeObserver =
    typeof ResizeObserver === "undefined"
      ? null
      : new ResizeObserver(function () {
          if (!feed.restoring && window.scrollY === feed.lastScroll) {
            feed.correctAnchor(feed.anchor);
          } else if (!feed.restoring) {
            feed.anchor = feed.visibleAnchor();
          }
        });
  bindMedia(wall, function () {
    feed.suspended = false;
    feed.considerLoading();
  });
  frame.addEventListener("click", function (event) {
    const link = event.target.closest("[data-feed-load]");
    if (link) {
      event.preventDefault();
      direction = link.dataset.feedLoad;
      load(direction, true);
    }
  });
  retry.addEventListener("click", function () {
    if (failedDirection) {
      load(failedDirection, true);
    }
  });
  window.addEventListener(
    "scroll",
    function () {
      if (raf !== null) {
        return;
      }
      raf = requestAnimationFrame(function () {
        raf = null;
        if (window.scrollY !== feed.lastScroll) {
          direction = window.scrollY >= feed.lastScroll ? "older" : "newer";
          feed.lastScroll = window.scrollY;
        }
        feed.anchor = feed.visibleAnchor();
        feed.considerLoading();
      });
    },
    { passive: true },
  );
  return feed;
}

export function bindFeed() {
  const feed = createFeedWindow();
  if (!feed) {
    return;
  }
  bindFeedNavigation(feed);
  bindFeedUpdates(feed);
  feed.updateControls();
  feed.cards().forEach(function (card) {
    if (feed.resizeObserver) {
      feed.resizeObserver.observe(card);
    }
  });
  restoreReadingPosition(feed);
}
