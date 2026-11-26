import { byId } from "./dom.js";
import { retainedInteraction } from "./feed-cards.js";

const stateKey = "myfeedPosition";

export function bindFeedNavigation(feed) {
  feed.savePosition = function () {
    const saved = feed.visibleAnchor();
    if (!saved || feed.restoring || feed.leaving) {
      return;
    }
    history.replaceState(
      Object.assign({}, history.state || {}, {
        [stateKey]: {
          id: saved.id.slice(5),
          offset: saved.offset,
          ceiling: feed.ceiling,
        },
      }),
      "",
    );
  };

  feed.restore = async function (saved) {
    if (feed.request) {
      feed.request.abort();
      feed.request = null;
    }
    feed.generation += 1;
    const activeGeneration = feed.generation;
    feed.restoring = true;
    feed.suspended = false;
    const activeRequest = new AbortController();
    feed.request = activeRequest;
    feed.status.textContent = feed.controller.dataset.loading;
    try {
      const result = await feed.fetchWindow(
        "/?around=" +
          saved.id +
          (saved.ceiling ? "&ceiling=" + saved.ceiling : ""),
        activeRequest.signal,
      );
      if (feed.generation !== activeGeneration) {
        return;
      }
      const oldCards = feed.cards();
      if (oldCards.some(retainedInteraction)) {
        feed.status.textContent = feed.controller.dataset.blocked;
        feed.suspended = true;
        return;
      }
      feed.evict(oldCards);
      const nextCards = Array.from(result.frame.querySelectorAll("article"));
      feed.wall.replaceChildren(...nextCards);
      feed.groups = [nextCards];
      feed.topHeight = 0;
      feed.bottomHeight = 0;
      feed.ceiling = result.frame.dataset.ceiling;
      feed.hasOlder = result.frame.dataset.hasOlder === "true";
      feed.hasNewer = result.frame.dataset.hasNewer === "true";
      feed.status.textContent = result.missing
        ? feed.controller.dataset.missing
        : "";
      feed.setSpacers();
      feed.updateControls();
      const target = byId("post-" + saved.id) || nextCards[0];
      if (target) {
        target.scrollIntoView({ block: "start", behavior: "instant" });
        window.scrollBy({
          top: -(Number.isFinite(saved.offset) ? saved.offset : 0),
          behavior: "instant",
        });
      }
      feed.anchor = feed.visibleAnchor();
      nextCards.forEach(function (card) {
        if (feed.resizeObserver) {
          feed.resizeObserver.observe(card);
        }
      });
    } catch (error) {
      if (error.name !== "AbortError") {
        console.error("MyFeed position restoration failed", error);
        feed.status.textContent = feed.controller.dataset.error;
        feed.suspended = true;
        // Keep a normal navigation available if restoring the window fails.
        feed.newerLink.href = "/?around=" + saved.id;
        feed.newerLink.hidden = false;
        feed.newerLink.dataset.restore = saved.id;
      }
    } finally {
      if (feed.generation === activeGeneration) {
        feed.restoring = false;
      }
      if (feed.request === activeRequest) {
        feed.request = null;
      }
      feed.lastScroll = window.scrollY;
    }
  };

  feed.frame.addEventListener(
    "click",
    function (event) {
      const link = event.target.closest("[data-restore]");
      if (link) {
        event.preventDefault();
        event.stopImmediatePropagation();
        feed.restore({ id: link.dataset.restore, offset: 0 });
        delete link.dataset.restore;
      }
    },
    true,
  );
  document.addEventListener(
    "click",
    function (event) {
      const link = event.target.closest("a[href]");
      if (link && !link.matches("[data-feed-load], [data-comments-load]")) {
        feed.savePosition();
        if (
          link.origin === location.origin &&
          link.pathname !== location.pathname
        ) {
          feed.leaving = true;
        }
      }
    },
    true,
  );
  document.addEventListener(
    "submit",
    function (event) {
      feed.savePosition();
      if (!event.target.matches("[data-clitorine-ajax]")) {
        feed.leaving = true;
      }
    },
    true,
  );
  window.addEventListener("pagehide", function () {
    feed.savePosition();
    if (feed.request) {
      feed.request.abort();
    }
  });
  window.addEventListener("pageshow", function (event) {
    if (event.persisted) {
      feed.leaving = false;
      feed.request = null;
      feed.restoring = false;
      feed.anchor = feed.visibleAnchor();
      feed.updateControls();
    }
  });
  window.addEventListener("popstate", function () {
    const saved = history.state && history.state[stateKey];
    if (saved) {
      feed.restore(saved);
    }
  });
  window.addEventListener("hashchange", function () {
    const match = /^#post-([1-9]\d*)$/.exec(location.hash);
    if (match) {
      feed.restore({ id: match[1], offset: 0 });
    }
  });
}

export function restoreReadingPosition(feed) {
  const hash = /^#post-([1-9]\d*)$/.exec(location.hash);
  const navigation = performance.getEntriesByType("navigation")[0];
  const saved = history.state && history.state[stateKey];
  if (
    saved &&
    navigation &&
    (navigation.type === "back_forward" || navigation.type === "reload")
  ) {
    feed.restore(saved);
  } else if (hash) {
    feed.restore({ id: hash[1], offset: 0 });
  } else {
    feed.anchor = feed.visibleAnchor();
    feed.considerLoading();
  }
}
