(() => {
  const root = document.querySelector("[data-chat-conversation-id]");
  if (!root) return;

  const conversationId = root.getAttribute("data-chat-conversation-id");
  let resolvedConversationId = conversationId;
  const canSend = root.getAttribute("data-chat-can-send") === "1";

  const messagesContainer = document.getElementById("chat-messages-list");
  const composer = document.getElementById("chat-composer");
  const input = document.getElementById("chat-input");
  const sendBtn = document.getElementById("chat-send");
  const onlineIndicator = document.getElementById("chat-online-indicator");
  const offlineNotice = document.getElementById("chat-offline-notice");

  const setStatus = (text) => {
    const isOnline = text === "online";

    // Header indicator (red dot when offline)
    if (onlineIndicator) {
      onlineIndicator.classList.toggle("hidden", isOnline);
    }

    // Show composer when online, offline notice when offline
    if (composer) composer.classList.toggle("hidden", !isOnline);
    if (offlineNotice) offlineNotice.classList.toggle("hidden", isOnline);
  };

  root.setAttribute("data-chat-js", "1");

  const escapeHtml = (s) =>
    String(s)
      .replaceAll("&", "&amp;")
      .replaceAll("<", "&lt;")
      .replaceAll(">", "&gt;")
      .replaceAll('"', "&quot;")
      .replaceAll("'", "&#039;");

  const renderMessage = (m) => {
    const isMe = m.is_me === true;
    const bubble =
      "border rounded-[22px] px-4 py-3 max-w-[88%] shadow-sm " +
      (isMe
        ? "bg-goamet-blue/20 border-goamet-blue/30"
        : "bg-white/5 border-white/10");
    const row = document.createElement("div");
    row.className = "flex " + (isMe ? "justify-end" : "justify-start");
    row.innerHTML = `
      <div class="${bubble}">
        <div class="text-[13px] font-semibold text-white leading-snug whitespace-pre-wrap">${escapeHtml(
          m.content || m.fallback || ""
        )}</div>
        <div class="mt-2 flex items-center justify-end gap-2 text-[10px] font-bold text-white/45">
          <span>${escapeHtml(m.created_at || "")}</span>
        </div>
      </div>
    `;
    return row;
  };

  const scrollToBottom = () => {
    if (!messagesContainer) return;
    messagesContainer.scrollIntoView({ block: "end" });
    window.scrollTo({ top: document.body.scrollHeight, behavior: "smooth" });
  };

  const normalizeApiMessage = (m, currentUserId) => {
    const t = (m.message_type || "").toLowerCase();
    return {
      id: m.id,
      content: m.content,
      fallback: t && t !== "text" ? `(${t})` : "",
      created_at: (m.created_at || "").replace("T", " ").replace("Z", ""),
      is_me: currentUserId && m.sender_id === currentUserId,
    };
  };

  const loadOnlineMessages = async () => {
    try {
      const res = await fetch(
        `/api/chat/conversations/${resolvedConversationId}/messages?limit=50`,
        { headers: { Accept: "application/json" } }
      );
      if (!res.ok) {
        let detail = "";
        try {
          const err = await res.json();
          detail = err?.code || err?.error || "";
        } catch {}
        setStatus(`offline (${res.status}${detail ? ` ${detail}` : ""})`);
        return;
      }
      const data = await res.json();
      const msgs = Array.isArray(data.messages) ? data.messages : [];
      const currentUserId = root.getAttribute("data-chat-current-user-id") || "";

      if (!msgs.length || !messagesContainer) return;

      const rendered = [];
      for (const m of msgs) {
        const n = normalizeApiMessage(m, currentUserId);
        rendered.push(renderMessage(n));
      }
      if (!rendered.length) return;

      messagesContainer.innerHTML = "";
      for (const el of rendered) messagesContainer.appendChild(el);
      scrollToBottom();
    } catch {
      setStatus("offline");
    }
  };

  const connectWs = async () => {
    try {
      const ticketRes = await fetch("/api/chat/ws-ticket", {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify({ conversation_id: resolvedConversationId }),
      });
      if (!ticketRes.ok) {
        let detail = "";
        try {
          const err = await ticketRes.json();
          detail = err?.code || err?.error || "";
        } catch {}
        setStatus(`offline (${ticketRes.status}${detail ? ` ${detail}` : ""})`);
        return;
      }
      const ticket = await ticketRes.json();
      const wsUrl = ticket.ws_url;
      if (!wsUrl) {
        setStatus("offline");
        return;
      }

      const ws = new WebSocket(wsUrl);
      ws.onopen = () => setStatus("online");
      ws.onclose = () => setStatus("offline");
      ws.onerror = () => setStatus("offline");
      ws.onmessage = (ev) => {
        try {
          const msg = JSON.parse(ev.data);
          if (msg.type === "new_message" && msg.message) {
            const currentUserId =
              root.getAttribute("data-chat-current-user-id") || "";
            const n = normalizeApiMessage(msg.message, currentUserId);
            if (messagesContainer) {
              messagesContainer.appendChild(renderMessage(n));
              scrollToBottom();
            }
          }
        } catch {
          // ignore
        }
      };
    } catch {
      setStatus("offline");
    }
  };

  const send = async () => {
    if (!canSend) return;
    const text = (input?.value || "").trim();
    if (!text) return;
    input.value = "";
    sendBtn.disabled = true;
    try {
      const res = await fetch(
        `/api/chat/conversations/${resolvedConversationId}/messages`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json", Accept: "application/json" },
          body: JSON.stringify({ content: text }),
        }
      );
      if (!res.ok) return;
      // WS will append; if WS isn't connected, append immediately.
      const data = await res.json();
      if (messagesContainer && data && data.id) {
        const currentUserId = root.getAttribute("data-chat-current-user-id") || "";
        const n = normalizeApiMessage(data, currentUserId);
        messagesContainer.appendChild(renderMessage(n));
        scrollToBottom();
      }
    } finally {
      sendBtn.disabled = false;
    }
  };

  if (composer && input && sendBtn) {
    if (!canSend) {
      input.disabled = true;
      sendBtn.disabled = true;
    } else {
      sendBtn.addEventListener("click", send);
      input.addEventListener("keydown", (e) => {
        if (e.key === "Enter" && !e.shiftKey) {
          e.preventDefault();
          send();
        }
      });
    }
  }

  // Prefer online view when available; keep offline cache as fallback.
  const resolveConversation = async () => {
    try {
      const res = await fetch(
        `/api/chat/resolve-conversation?local_conversation_id=${encodeURIComponent(
          conversationId
        )}`,
        { headers: { Accept: "application/json" } }
      );
      if (!res.ok) {
        let detail = "";
        try {
          const err = await res.json();
          detail = err?.error || err?.code || "";
        } catch {}
        setStatus(`offline (${res.status}${detail ? ` ${detail}` : ""})`);
        return false;
      }
      const data = await res.json();
      if (data && data.chat_conversation_id) {
        resolvedConversationId = data.chat_conversation_id;
        return true;
      }
      return false;
    } catch {
      setStatus("offline");
      return false;
    }
  };

  resolveConversation().then((ok) => {
    if (!ok) return;
    loadOnlineMessages();
    connectWs();
  });
})();
