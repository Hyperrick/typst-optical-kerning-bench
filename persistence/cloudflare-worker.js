export default {
  async fetch(request, env) {
    const headers = corsHeaders(env);

    if (request.method === "OPTIONS") {
      return new Response(null, { headers });
    }

    const url = new URL(request.url);
    const path = normalizePath(url.pathname);

    if (request.method === "GET" && (path === "/" || path === "/health")) {
      return json({ ok: true, service: "optikern-preference-api" }, 200, headers);
    }

    if (request.method === "GET" && path === "/results") {
      return handleResults(env, headers);
    }

    if (request.method === "POST" && (path === "/" || path === "/submit")) {
      return handleSubmit(request, env, headers);
    }

    if ((request.method === "POST" || request.method === "DELETE") && path === "/admin/reset") {
      return handleReset(request, env, headers);
    }

    return json({ ok: false, error: "not_found" }, 404, headers);
  },
};

async function handleSubmit(request, env, headers) {
  let payload;
  try {
    payload = await request.json();
  } catch {
    return json({ ok: false, error: "invalid_json" }, 400, headers);
  }

  if (!payload || !Array.isArray(payload.votes)) {
    return json({ ok: false, error: "invalid_payload" }, 400, headers);
  }

  const participantId = cleanId(payload.participantId) || crypto.randomUUID();
  const id = crypto.randomUUID();
  const participantKey = `participant:${participantId}`;
  const previous = await env.VOTES.get(participantKey, { type: "json" });
  const fingerprintHash = await requestFingerprintHash(request, env);
  const fingerprintKey = fingerprintHash ? `fingerprint:${fingerprintHash}` : null;
  const previousFingerprintParticipant = fingerprintKey
    ? await env.VOTES.get(fingerprintKey)
    : null;
  const duplicateReason = previous
    ? "same_participant_id"
    : previousFingerprintParticipant && previousFingerprintParticipant !== participantId
      ? "same_browser_fingerprint"
      : null;
  const excludedFromPublicResults = duplicateReason === "same_browser_fingerprint";
  const stored = {
    ...payload,
    participantId,
    serverReceivedAt: new Date().toISOString(),
    serverId: id,
    duplicate: Boolean(duplicateReason),
    duplicateOf: previous?.serverId || null,
    duplicateReason,
    excludedFromPublicResults,
    fingerprintHash,
  };

  await env.VOTES.put(`submission:${id}`, JSON.stringify(stored), {
    metadata: {
      participantId,
      completed: Number(payload.completed || 0),
      selectedTrialCount: Number(payload.selectedTrialCount || payload.trialCount || 0),
      duplicate: Boolean(duplicateReason),
      excludedFromPublicResults,
    },
  });
  await env.VOTES.put(participantKey, JSON.stringify(stored));
  if (fingerprintKey && !previousFingerprintParticipant) {
    await env.VOTES.put(fingerprintKey, participantId);
  }

  return json(
    {
      ok: true,
      id,
      participantId,
      duplicate: Boolean(duplicateReason),
      duplicateReason,
      duplicateOf: previous?.serverId || null,
      excludedFromPublicResults,
    },
    200,
    headers,
  );
}

async function handleResults(env, headers) {
  const records = await readJsonPrefix(env.VOTES, "participant:");
  const submissions = await countPrefix(env.VOTES, "submission:");
  const stats = aggregateParticipantRecords(records);
  return json(
    {
      ok: true,
      generatedAt: new Date().toISOString(),
      storage: "cloudflare-workers-kv",
      submissions: {
        total: submissions,
        participants: records.length,
        completed: stats.completed,
        included: stats.included,
        incomplete: stats.incomplete,
        excludedDuplicates: stats.excludedDuplicates,
      },
      votes: {
        included: stats.includedVotes,
      },
      modes: stats.modes,
      headToHead: stats.headToHead,
      samples: stats.samples,
    },
    200,
    noStore(headers),
  );
}

async function handleReset(request, env, headers) {
  const auth = adminAuth(request, env);
  if (!auth.ok) {
    return json({ ok: false, error: auth.error }, auth.status, headers);
  }

  const prefixes = ["submission:", "participant:", "fingerprint:"];
  const deleted = {};
  for (const prefix of prefixes) {
    deleted[prefix] = await deletePrefix(env.VOTES, prefix);
  }
  return json(
    {
      ok: true,
      resetAt: new Date().toISOString(),
      deleted,
      totalDeleted: Object.values(deleted).reduce((sum, count) => sum + count, 0),
    },
    200,
    headers,
  );
}

function aggregateParticipantRecords(records) {
  const modes = new Map();
  const headToHead = new Map();
  const samples = new Map();
  let completed = 0;
  let included = 0;
  let incomplete = 0;
  let excludedDuplicates = 0;
  let includedVotes = 0;

  for (const record of records) {
    const votes = Array.isArray(record.votes) ? record.votes : [];
    const selected = Number(record.selectedTrialCount || record.trialCount || votes.length || 0);
    const done = Number(record.completed || votes.length || 0);
    const isComplete = done >= selected && selected > 0;
    if (isComplete) {
      completed += 1;
    } else {
      incomplete += 1;
      continue;
    }
    if (record.excludedFromPublicResults || record.duplicateReason === "same_browser_fingerprint") {
      excludedDuplicates += 1;
      continue;
    }

    included += 1;
    for (const vote of votes) {
      const winner = cleanMode(vote.winner);
      const losers = Array.isArray(vote.losers)
        ? vote.losers.map(cleanMode).filter(Boolean)
        : cleanMode(vote.loser)
          ? [cleanMode(vote.loser)]
          : [];
      const shownModes = Array.isArray(vote.shownModes)
        ? vote.shownModes.map(cleanMode).filter(Boolean)
        : [winner, ...losers].filter(Boolean);
      if (!winner || losers.length === 0) continue;

      includedVotes += 1;
      for (const mode of new Set(shownModes)) {
        ensureMode(modes, mode).appearances += 1;
      }
      ensureMode(modes, winner).wins += losers.length;
      for (const loser of losers) {
        if (loser === winner) continue;
        ensureMode(modes, loser).losses += 1;
        const key = `${winner}\u0000${loser}`;
        headToHead.set(key, {
          winner,
          loser,
          wins: (headToHead.get(key)?.wins || 0) + 1,
        });
      }
      const sampleKey = [vote.kind, vote.fontId, vote.sample].map((value) => value || "").join("\u0000");
      const sample = samples.get(sampleKey) || {
        kind: vote.kind || "",
        fontId: vote.fontId || "",
        family: vote.family || "",
        sample: vote.sample || "",
        votes: 0,
      };
      sample.votes += 1;
      samples.set(sampleKey, sample);
    }
  }

  return {
    completed,
    included,
    incomplete,
    excludedDuplicates,
    includedVotes,
    modes: [...modes.values()]
      .map((mode) => ({
        ...mode,
        winRate: mode.wins + mode.losses > 0 ? mode.wins / (mode.wins + mode.losses) : 0,
      }))
      .sort((a, b) => b.winRate - a.winRate || b.wins - a.wins || a.mode.localeCompare(b.mode)),
    headToHead: [...headToHead.values()].sort((a, b) => b.wins - a.wins),
    samples: [...samples.values()].sort((a, b) => b.votes - a.votes).slice(0, 30),
  };
}

function ensureMode(map, mode) {
  if (!map.has(mode)) {
    map.set(mode, { mode, appearances: 0, wins: 0, losses: 0 });
  }
  return map.get(mode);
}

async function readJsonPrefix(namespace, prefix) {
  const records = [];
  let cursor;
  do {
    const listed = await namespace.list({ prefix, cursor, limit: 1000 });
    await Promise.all(
      listed.keys.map(async (key) => {
        const value = await namespace.get(key.name, { type: "json" });
        if (value) records.push(value);
      }),
    );
    cursor = listed.cursor;
    if (listed.list_complete) break;
  } while (cursor);
  return records;
}

async function countPrefix(namespace, prefix) {
  let count = 0;
  let cursor;
  do {
    const listed = await namespace.list({ prefix, cursor, limit: 1000 });
    count += listed.keys.length;
    cursor = listed.cursor;
    if (listed.list_complete) break;
  } while (cursor);
  return count;
}

async function deletePrefix(namespace, prefix) {
  let deleted = 0;
  let cursor;
  do {
    const listed = await namespace.list({ prefix, cursor, limit: 1000 });
    await Promise.all(listed.keys.map((key) => namespace.delete(key.name)));
    deleted += listed.keys.length;
    cursor = listed.cursor;
    if (listed.list_complete) break;
  } while (cursor);
  return deleted;
}

function adminAuth(request, env) {
  const expected = env.RESET_TOKEN;
  if (!expected) {
    return { ok: false, status: 503, error: "reset_token_not_configured" };
  }
  const auth = request.headers.get("authorization") || "";
  const bearer = auth.startsWith("Bearer ") ? auth.slice("Bearer ".length).trim() : "";
  const headerToken = request.headers.get("x-reset-token") || "";
  const provided = bearer || headerToken;
  if (provided !== expected) {
    return { ok: false, status: 401, error: "unauthorized" };
  }
  return { ok: true };
}

function cleanId(value) {
  if (typeof value !== "string") return null;
  const trimmed = value.trim();
  if (!/^[A-Za-z0-9._:-]{8,120}$/.test(trimmed)) return null;
  return trimmed;
}

function cleanMode(value) {
  if (typeof value !== "string") return null;
  const trimmed = value.trim();
  if (!/^[A-Za-z0-9._:-]{2,80}$/.test(trimmed)) return null;
  return trimmed;
}

async function requestFingerprintHash(request, env) {
  const ip = request.headers.get("cf-connecting-ip") || "";
  const userAgent = request.headers.get("user-agent") || "";
  const acceptLanguage = request.headers.get("accept-language") || "";
  const salt = env.FINGERPRINT_SALT || "optikern-v1";
  const raw = `${salt}|${ip}|${userAgent}|${acceptLanguage}`;
  const bytes = new TextEncoder().encode(raw);
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return [...new Uint8Array(digest)]
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

function normalizePath(pathname) {
  if (pathname.length > 1 && pathname.endsWith("/")) {
    return pathname.slice(0, -1);
  }
  return pathname;
}

function corsHeaders(env) {
  return {
    "access-control-allow-origin": env.ALLOWED_ORIGIN || "*",
    "access-control-allow-methods": "GET, POST, DELETE, OPTIONS",
    "access-control-allow-headers": "authorization, content-type, x-reset-token",
    "content-type": "application/json; charset=utf-8",
  };
}

function noStore(headers) {
  return {
    ...headers,
    "cache-control": "no-store",
  };
}

function json(payload, status, headers) {
  return new Response(JSON.stringify(payload), { status, headers });
}
