import assert from "node:assert/strict";
import { test } from "node:test";

import { bindInteractions, opensImagePreview } from "../assets/clitorine/interactions.js";

test("post images open in the modal and restore focus on close", () => {
  const listeners = {};
  const image = {
    removeAttribute(name) { if (name === "src") delete this.src; },
  };
  const modal = {
    querySelector: () => image,
    addEventListener(name, callback) { listeners[name] = callback; },
  };
  const link = {
    href: "https://example.test/posts/4/image",
    isConnected: true,
    matches: (selector) => selector === "[data-image-preview]",
    querySelector: () => ({ alt: "posted image" }),
    focus() { this.focused = true; },
  };
  globalThis.document = {
    getElementById: (id) => id === "image-modal" ? modal : null,
    addEventListener(name, callback) {
      (listeners[name] ??= []).push(callback);
    },
  };
  globalThis.bootstrap = {
    Modal: { getOrCreateInstance: () => ({ show: (opener) => { modal.opener = opener; } }) },
  };
  try {
    bindInteractions();
    const event = {
      target: { closest: (selector) => selector === "a[data-image-preview]" ? link : null },
      button: 0,
      preventDefault() { this.defaultPrevented = true; },
    };
    assert.equal(opensImagePreview({ ...event, ctrlKey: true }, link), false);
    for (const listener of listeners.click) listener(event);
    assert.equal(event.defaultPrevented, true);
    assert.equal(modal.opener, link);
    assert.equal(image.src, link.href);
    assert.equal(image.alt, "posted image");
    listeners["hidden.bs.modal"]();
    assert.equal(image.src, undefined);
    assert.equal(link.focused, true);
    delete globalThis.bootstrap;
    assert.equal(opensImagePreview({ ...event, defaultPrevented: false }, link), false);
  } finally {
    delete globalThis.document;
    delete globalThis.bootstrap;
  }
});
