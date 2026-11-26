import { hasComposerDraft } from "./composer.js";
import { byId } from "./dom.js";
import { protectedCard } from "./feed-cards.js";

export function bindFeedUpdates(feed) {
  const newPosts = byId("feed-new-posts");
  const hour = 60 * 60 * 1000;
  let pollTimer = null;
  let pollRequest = null;
  let lastPoll = Date.now();

  function clearPoll() {
    clearTimeout(pollTimer);
    pollTimer = null;
  }

  function schedulePoll() {
    clearPoll();
    if (!document.hidden) {
      pollTimer = setTimeout(
        checkNewPosts,
        Math.max(0, hour - (Date.now() - lastPoll)),
      );
    }
  }

  async function checkNewPosts() {
    if (document.hidden || pollRequest) {
      return;
    }
    lastPoll = Date.now();
    pollRequest = new AbortController();
    try {
      const response = await fetch("/feed/latest", {
        signal: pollRequest.signal,
        credentials: "same-origin",
      });
      if (!response.ok) {
        throw new Error("HTTP " + response.status);
      }
      const latest = (await response.text()).trim();
      if (!/^\d+$/.test(latest)) {
        throw new Error("Invalid latest post id");
      }
      newPosts.hidden = BigInt(latest) <= BigInt(feed.ceiling);
    } catch (error) {
      if (error.name !== "AbortError") {
        console.error("MyFeed new-post check failed", error);
        feed.status.textContent = feed.controller.dataset.error;
      }
    } finally {
      pollRequest = null;
      schedulePoll();
    }
  }

  document.addEventListener("visibilitychange", function () {
    if (document.hidden) {
      clearPoll();
      feed.savePosition();
      if (pollRequest) {
        pollRequest.abort();
      }
    } else {
      schedulePoll();
    }
  });
  newPosts.addEventListener("click", function (event) {
    if (feed.cards().some(protectedCard) || hasComposerDraft()) {
      event.preventDefault();
      feed.status.textContent = feed.controller.dataset.blocked;
    } else {
      history.replaceState(null, "");
    }
  });
  window.addEventListener("pagehide", function () {
    clearPoll();
    if (pollRequest) {
      pollRequest.abort();
    }
  });
  window.addEventListener("pageshow", schedulePoll);
  schedulePoll();
}
