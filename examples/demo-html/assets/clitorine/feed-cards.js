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
    })
  );
}

export function protectedCard(card) {
  const active = document.activeElement;
  const rect = card.getBoundingClientRect();
  const offscreenPlayer =
    active &&
    active.matches("iframe") &&
    (rect.bottom <= 0 || rect.top >= window.innerHeight);
  return (
    (card.contains(active) && !offscreenPlayer) || retainedInteraction(card)
  );
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
