import { byId, feedLabels } from "./dom.js";

function showInlineNotice(anchor, message) {
  if (!anchor) {
    return;
  }
  let notice = anchor.querySelector("[data-clitorine-notice]");
  if (!notice) {
    notice = document.createElement("div");
    notice.setAttribute("data-clitorine-notice", "1");
    notice.setAttribute("role", "status");
    anchor.appendChild(notice);
  }
  notice.className = "alert alert-success mt-2 mb-0 py-2 small";
  notice.textContent = message;
}

function bindAjaxForms() {
  document.addEventListener("submit", function (event) {
    const form = event.target.closest("form[data-clitorine-ajax]");
    if (!form) {
      return;
    }
    event.preventDefault();
    if (form.dataset.busy === "1") {
      return;
    }
    form.dataset.busy = "1";
    const kind = form.getAttribute("data-clitorine-ajax") || "";
    const params = new URLSearchParams(new FormData(form));
    fetch(form.action, {
      method: "POST",
      body: params.toString(),
      credentials: "same-origin",
      headers: {
        "X-MyFeed-Ajax": "1",
        "Content-Type": "application/x-www-form-urlencoded",
      },
    })
      .then(function (response) {
        if (!response.ok) {
          throw new Error("request failed");
        }
        return response.text();
      })
      .then(function (text) {
        if (kind === "like") {
          const article = form.closest("article");
          const badge = article && article.querySelector("[data-like-count]");
          const count = Number.parseInt(text.trim(), 10);
          if (!Number.isFinite(count) || count <= 0 || !badge) {
            throw new Error("Invalid like response");
          }
          badge.textContent = String(count) + " likes";
          return;
        }
        if (kind === "comment") {
          const wrap = form.parentElement;
          form.reset();
          showInlineNotice(
            wrap,
            text.trim() || "Comment sent. It will appear after approval.",
          );
        }
      })
      .catch(function (error) {
        console.error("MyFeed action failed", error);
        const labels = feedLabels();
        showInlineNotice(form.parentElement, labels.dataset.error);
        const notice = form.parentElement.querySelector(
          "[data-clitorine-notice]",
        );
        if (notice) {
          notice.className = "alert alert-danger mt-2 mb-0 py-2 small";
        }
      })
      .finally(function () {
        delete form.dataset.busy;
      });
  });
}

function bindDeleteModal() {
  const modalEl = byId("delete-post-modal");
  const confirmForm = byId("delete-post-confirm-form");
  const csrfSlot = byId("delete-post-csrf");
  if (
    !modalEl ||
    !confirmForm ||
    !csrfSlot ||
    typeof bootstrap === "undefined"
  ) {
    return;
  }
  document.addEventListener("click", function (event) {
    const btn = event.target.closest("[data-clitorine-delete]");
    if (!btn) {
      return;
    }
    const action = btn.getAttribute("data-delete-action") || "";
    const token = btn.getAttribute("data-delete-token") || "";
    confirmForm.setAttribute("action", action);
    csrfSlot.innerHTML =
      '<input type="hidden" name="_token" value="' +
      token.replace(/"/g, "&quot;") +
      '" />';
    bootstrap.Modal.getOrCreateInstance(modalEl).show();
  });
}

function bindComments() {
  document.addEventListener("click", async function (event) {
    const link = event.target.closest("[data-comments-load]");
    if (!link) {
      return;
    }
    event.preventDefault();
    const article = link.closest("article");
    if (article.dataset.commentsBusy === "1") {
      return;
    }
    article.dataset.commentsBusy = "1";
    const status = article.querySelector("[data-comment-status]");
    const labels = feedLabels();
    status.textContent = labels.dataset.loading;
    try {
      const response = await fetch(link.href, {
        headers: { "X-MyFeed-Ajax": "1" },
        credentials: "same-origin",
      });
      if (!response.ok) {
        throw new Error("HTTP " + response.status);
      }
      const template = document.createElement("template");
      template.innerHTML = await response.text();
      const next = template.content.querySelector("[data-comment-window]");
      if (!next || next.querySelectorAll("[data-comment-id]").length > 10) {
        throw new Error("Invalid comment window");
      }
      const old = article.querySelector("[data-comment-window]");
      const offset = link.getBoundingClientRect().top;
      old.replaceWith(next);
      const control = next.querySelector("[data-comments-load]");
      if (control) {
        control.focus({ preventScroll: true });
        window.scrollBy({
          top: control.getBoundingClientRect().top - offset,
          behavior: "instant",
        });
      }
      status.textContent = "";
    } catch (error) {
      console.error("MyFeed comment loading failed", error);
      status.textContent = labels.dataset.error;
    } finally {
      delete article.dataset.commentsBusy;
    }
  });
}

export function bindInteractions() {
  bindAjaxForms();
  bindDeleteModal();
  bindComments();
}
