export function byId(id) {
  return document.getElementById(id);
}

export function feedLabels() {
  return (
    byId("feed-controller") || document.querySelector("[data-feed-labels]")
  );
}
