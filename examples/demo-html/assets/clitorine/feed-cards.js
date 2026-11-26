export function visibleAnchor(wall) {
  const card = Array.from(wall.querySelectorAll("article")).find(
    function (node) {
      const rect = node.getBoundingClientRect();
      return rect.bottom > 0 && rect.top < window.innerHeight;
    },
  );
  return card
    ? { id: card.id, offset: card.getBoundingClientRect().top }
    : null;
}

export function retainedInteraction(card) {
  return (
    card.dataset.commentsBusy === "1" ||
    card.querySelector('[data-busy="1"], [data-comments-busy="1"]') ||
    Array.from(card.querySelectorAll("textarea")).some(function (input) {
      return input.value.trim() !== "";
    }) ||
    Array.from(card.querySelectorAll("video")).some(function (video) {
      return !video.paused;
    }) ||
    card.dataset.mediaPinned === "1"
  );
}

export function protectedCard(card) {
  return card.contains(document.activeElement) || retainedInteraction(card);
}

export function protectedGroup(group) {
  return group.some(function (card) {
    const rect = card.getBoundingClientRect();
    return (
      protectedCard(card) || (rect.bottom > 0 && rect.top < window.innerHeight)
    );
  });
}

export function groupHeight(wall, group) {
  if (!group.length) {
    return 0;
  }
  const first = group[0].getBoundingClientRect();
  const last = group[group.length - 1].getBoundingClientRect();
  return (
    last.bottom - first.top + Number.parseFloat(getComputedStyle(wall).rowGap)
  );
}

export function evict(group, resizeObserver) {
  group.forEach(function (card) {
    if (resizeObserver) {
      resizeObserver.unobserve(card);
    }
    card.querySelectorAll("video").forEach(function (video) {
      video.pause();
      video.removeAttribute("src");
      video.load();
    });
    card.querySelectorAll("iframe").forEach(function (iframe) {
      iframe.src = "about:blank";
    });
    card.remove();
  });
}

export function bindMedia(wall, onRelease) {
  // Cross-origin players do not expose their playback state; focus pins them.
  wall.addEventListener("focusin", function (event) {
    if (event.target.matches("iframe")) {
      event.target.closest("article").dataset.mediaPinned = "1";
    }
  });
  window.addEventListener("blur", function () {
    const focused = document.activeElement;
    if (focused && focused.matches("#wall iframe")) {
      focused.closest("article").dataset.mediaPinned = "1";
    }
  });
  wall.addEventListener("click", function (event) {
    const release = event.target.closest("[data-release-media]");
    if (release) {
      delete release.closest("article").dataset.mediaPinned;
      release.blur();
      onRelease();
    }
  });
}
