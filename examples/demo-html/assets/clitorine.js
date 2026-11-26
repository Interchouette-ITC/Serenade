/*!
 * Clitorine - light UX helpers beside Bootstrap + Quill (MyFeed demo only).
 * Edit the modules in clitorine/ and rebuild with npm run build.
 */
"use strict";

import { bindComposer } from "./clitorine/composer.js";
import { bindInteractions } from "./clitorine/interactions.js";
import { bindFeed } from "./clitorine/feed.js";

function bind() {
  bindComposer();
  bindInteractions();
  bindFeed();
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", bind);
} else {
  bind();
}
