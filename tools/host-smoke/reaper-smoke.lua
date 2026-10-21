-- MusicCraft plugin smoke test inside REAPER (docs/smoke-tests.md).
-- MC_LOG: log file. MC_DIR: output folder. MC_PHASE: "build" or "reopen".
local log = io.open(os.getenv("MC_LOG"), "a")
local dir = os.getenv("MC_DIR")
local phase = os.getenv("MC_PHASE")
local function say(...)
  log:write(table.concat({...}, " "), "\n")
  log:flush()
end

say("reaper", reaper.GetAppVersion(), "phase", phase)

local function render(name)
  reaper.GetSetProjectInfo_String(0, "RENDER_FILE", dir, true)
  reaper.GetSetProjectInfo_String(0, "RENDER_PATTERN", name, true)
  reaper.GetSetProjectInfo(0, "RENDER_SETTINGS", 0, true)
  reaper.GetSetProjectInfo(0, "RENDER_BOUNDSFLAG", 1, true)
  reaper.GetSetProjectInfo(0, "RENDER_SRATE", 48000, true)
  reaper.GetSetProjectInfo(0, "RENDER_CHANNELS", 2, true)
  reaper.Main_OnCommand(42230, 0) -- render with the last settings, close the dialog
  say("rendered", name)
end

-- Renders each track alone as <prefix><track name>, then calls `finish`.
-- After each mute change it waits half a second: REAPER fades a track in
-- over a few milliseconds when it is unmuted, and a render that starts
-- during that fade would begin quieter.
local function render_each(prefix, finish)
  local n = reaper.CountTracks(0)
  local t, since = 0, nil
  local function step()
    if t >= n then
      finish()
      return
    end
    if not since then
      for u = 0, n - 1 do
        reaper.SetMediaTrackInfo_Value(reaper.GetTrack(0, u), "B_MUTE", u == t and 0 or 1)
      end
      since = reaper.time_precise()
    elseif reaper.time_precise() - since >= 0.5 then
      local _, name = reaper.GetSetMediaTrackInfo_String(reaper.GetTrack(0, t), "P_NAME", "", false)
      render(prefix .. name:gsub(" ", "-"))
      t, since = t + 1, nil
    end
    reaper.defer(step)
  end
  step()
end

local function fx_summary(track, fx)
  local _, name = reaper.TrackFX_GetFXName(track, fx, "")
  return name .. " params=" .. reaper.TrackFX_GetNumParams(track, fx)
end

if phase == "build" then
  -- 1. Every MusicCraft plugin REAPER found, loaded once.
  local found = {}
  local i = 0
  while true do
    local ok, name, ident = reaper.EnumInstalledFX(i)
    if not ok then break end
    if name:find("MusicCraft", 1, true) then found[#found + 1] = name end
    i = i + 1
  end
  say("found", #found)
  local counts = { CLAP = 0, CLAPi = 0, VST3 = 0, VST3i = 0 }
  local scratch_index = reaper.CountTracks(0)
  reaper.InsertTrackAtIndex(scratch_index, false)
  local scratch = reaper.GetTrack(0, scratch_index)
  for _, name in ipairs(found) do
    local kind = name:match("^(%w+):")
    local fx = reaper.TrackFX_AddByName(scratch, name, false, -1)
    if fx < 0 then
      say("LOAD-FAILED", name)
    else
      counts[kind] = (counts[kind] or 0) + 1
      if reaper.TrackFX_GetNumParams(scratch, fx) < 2 then say("NO-PARAMS", name) end
      reaper.TrackFX_Delete(scratch, fx)
    end
  end
  for kind, n in pairs(counts) do say("loaded", kind, n) end
  reaper.DeleteTrack(scratch)

  -- 2. A synth, Antiserum and a delay play a chord, per format.
  checks = {}
  for _, fmt in ipairs({ "CLAP", "VST3" }) do
    for _, inst in ipairs({ "Synth", "Antiserum" }) do
      local index = reaper.CountTracks(0)
      reaper.InsertTrackAtIndex(index, false)
      local track = reaper.GetTrack(0, index)
      reaper.GetSetMediaTrackInfo_String(track, "P_NAME", fmt .. " " .. inst, true)
      local synth = reaper.TrackFX_AddByName(track, fmt .. "i: " .. inst .. " (MusicCraft)", false, -1)
      local delay = reaper.TrackFX_AddByName(track, fmt .. ": Delay (MusicCraft)", false, -1)
      say(fmt, inst, "fx", synth, delay)
      if synth >= 0 then say("  ", fx_summary(track, synth)) end
      if delay >= 0 then
        say("  ", fx_summary(track, delay))
        -- 3. A parameter set by the host reads back with our text.
        reaper.TrackFX_SetParamNormalized(track, delay, 1, 0.25)
        checks[#checks + 1] = { track = track, fx = delay, label = fmt .. " " .. inst }
      end
      -- 4. The editor opens and closes in a REAPER window.
      if synth >= 0 then
        reaper.TrackFX_Show(track, synth, 3)
        say("  editor open", tostring(reaper.TrackFX_GetFloatingWindow(track, synth) ~= nil))
        reaper.TrackFX_Show(track, synth, 2)
      end
      local item = reaper.CreateNewMIDIItemInProj(track, 0, 2, false)
      local take = reaper.GetActiveTake(item)
      for k, pitch in ipairs({ 60, 64, 67 }) do
        local s = reaper.MIDI_GetPPQPosFromProjTime(take, 0.0 + 0.1 * (k - 1))
        local e = reaper.MIDI_GetPPQPosFromProjTime(take, 1.5)
        reaper.MIDI_InsertNote(take, false, false, s, e, 0, pitch, 100, true)
      end
      reaper.MIDI_Sort(take)
      -- Solo-free: each track renders on its own below.
      reaper.SetMediaTrackInfo_Value(track, "B_MUTE", 1)
    end
  end
  -- 3b. After the audio engine has run a while, the host's values have
  -- reached the plugins and read back with our text. 5. Render, save.
  -- Waits until every value has arrived (the engine processes them), or 15 s.
  local start = reaper.time_precise()
  local function arrived()
    for _, c in ipairs(checks) do
      if math.abs(reaper.TrackFX_GetParamNormalized(c.track, c.fx, 1) - 0.25) > 1e-4 then
        return false
      end
    end
    return true
  end
  local function after()
    if not arrived() and reaper.time_precise() - start < 15 then
      reaper.defer(after)
      return
    end
    say("values arrived after", string.format("%.1f s", reaper.time_precise() - start))
    for _, c in ipairs(checks) do
      local _, text = reaper.TrackFX_GetFormattedParamValue(c.track, c.fx, 1, "")
      local _, pname = reaper.TrackFX_GetParamName(c.track, c.fx, 1, "")
      say("param", c.label, pname, "=", text, reaper.TrackFX_GetParamNormalized(c.track, c.fx, 1))
    end
    render_each("first-", function()
      reaper.Main_SaveProjectEx(0, dir .. "/smoke.rpp", 0)
      say("saved")
      say("done")
      log:close()
    end)
  end
  reaper.defer(after)
  return
else
  -- Reopened: the plugins and their values came back; render again.
  reaper.Main_openProject(dir .. "/smoke.rpp")
  local n = reaper.CountTracks(0)
  say("tracks", n)
  for t = 0, n - 1 do
    local track = reaper.GetTrack(0, t)
    local _, name = reaper.GetSetMediaTrackInfo_String(track, "P_NAME", "", false)
    for fx = 0, reaper.TrackFX_GetCount(track) - 1 do
      say(name, fx_summary(track, fx))
    end
    if reaper.TrackFX_GetCount(track) > 1 then
      local _, text = reaper.TrackFX_GetFormattedParamValue(track, 1, 1, "")
      say("  param after reopen", text, reaper.TrackFX_GetParamNormalized(track, 1, 1))
    end
  end
  render_each("again-", function()
    say("done")
    log:close()
  end)
end
