-- Sidechain through REAPER's routing: a key track sent to channels 3/4 of a
-- track whose compressor (CLAP, then VST3) has its sidechain switch on or off.
local log = io.open(os.getenv("MC_LOG"), "a")
local dir = os.getenv("MC_DIR")
local function say(...) log:write(table.concat({...}, " "), "\n") log:flush() end

local function chord(track, pitches, length)
  local item = reaper.CreateNewMIDIItemInProj(track, 0, length, false)
  local take = reaper.GetActiveTake(item)
  for _, p in ipairs(pitches) do
    local s = reaper.MIDI_GetPPQPosFromProjTime(take, 0)
    local e = reaper.MIDI_GetPPQPosFromProjTime(take, length - 0.1)
    reaper.MIDI_InsertNote(take, false, false, s, e, 0, p, 120, true)
  end
  reaper.MIDI_Sort(take)
end

local function param_index(track, fx, word)
  for i = 0, reaper.TrackFX_GetNumParams(track, fx) - 1 do
    local _, name = reaper.TrackFX_GetParamName(track, fx, i, "")
    if name:lower():find(word) then return i, name end
  end
  return -1, "?"
end

local jobs = {}
for _, fmt in ipairs({ "CLAP", "VST3" }) do
  local base = reaper.CountTracks(0)
  reaper.InsertTrackAtIndex(base, false)
  reaper.InsertTrackAtIndex(base + 1, false)
  local target = reaper.GetTrack(0, base)
  local key = reaper.GetTrack(0, base + 1)
  reaper.SetMediaTrackInfo_Value(target, "I_NCHAN", 4)
  reaper.TrackFX_AddByName(target, fmt .. "i: Synth (MusicCraft)", false, -1)
  local comp = reaper.TrackFX_AddByName(target, fmt .. ": Compressor (MusicCraft)", false, -1)
  reaper.TrackFX_AddByName(key, fmt .. "i: Synth (MusicCraft)", false, -1)
  chord(target, { 48 }, 2)
  chord(key, { 36, 43, 48, 55 }, 2)
  -- The key reaches only channels 3/4 of the target, not the master.
  reaper.SetMediaTrackInfo_Value(key, "B_MAINSEND", 0)
  local send = reaper.CreateTrackSend(key, target)
  reaper.SetTrackSendInfo_Value(key, 0, send, "I_DSTCHAN", 2)
  local switch, name = param_index(target, comp, "sidechain")
  local ins = reaper.TrackFX_GetIOSize(target, comp)
  say(fmt, "compressor fx", comp, "inputs", ins, "switch", switch, name)
  jobs[#jobs + 1] = { fmt = fmt, target = target, key = key, comp = comp, switch = switch }
end

local function only(job)
  for t = 0, reaper.CountTracks(0) - 1 do
    local tr = reaper.GetTrack(0, t)
    reaper.SetMediaTrackInfo_Value(tr, "B_MUTE", (tr == job.target or tr == job.key) and 0 or 1)
  end
end

local function render(name)
  reaper.GetSetProjectInfo_String(0, "RENDER_FILE", dir, true)
  reaper.GetSetProjectInfo_String(0, "RENDER_PATTERN", name, true)
  reaper.GetSetProjectInfo(0, "RENDER_SETTINGS", 0, true)
  reaper.GetSetProjectInfo(0, "RENDER_BOUNDSFLAG", 1, true)
  reaper.GetSetProjectInfo(0, "RENDER_SRATE", 48000, true)
  reaper.GetSetProjectInfo(0, "RENDER_CHANNELS", 2, true)
  reaper.Main_OnCommand(42230, 0)
  say("rendered", name)
end

-- Renders after the engine has applied the switch.
local steps = {}
for _, job in ipairs(jobs) do
  for _, on in ipairs({ 0, 1 }) do
    steps[#steps + 1] = function()
      only(job)
      reaper.TrackFX_SetParamNormalized(job.target, job.comp, job.switch, on)
    end
    steps[#steps + 1] = function()
      local _, text = reaper.TrackFX_GetFormattedParamValue(job.target, job.comp, job.switch, "")
      say(job.fmt, "sidechain", text)
      render("sc-" .. job.fmt .. "-" .. (on == 1 and "on" or "off"))
    end
  end
  -- Control: the switch on, but the key's send muted.
  steps[#steps + 1] = function()
    reaper.SetTrackSendInfo_Value(job.key, 0, 0, "D_VOL", 0)
  end
  steps[#steps + 1] = function()
    render("sc-" .. job.fmt .. "-on-no-key")
    reaper.SetTrackSendInfo_Value(job.key, 0, 0, "D_VOL", 1)
    -- References: the target with the compressor bypassed, the key alone.
    reaper.TrackFX_SetEnabled(job.target, job.comp, false)
    render("sc-" .. job.fmt .. "-dry")
    reaper.TrackFX_SetEnabled(job.target, job.comp, true)
    reaper.SetMediaTrackInfo_Value(job.target, "B_MUTE", 1)
    reaper.SetMediaTrackInfo_Value(job.key, "B_MAINSEND", 1)
    render("sc-" .. job.fmt .. "-key")
    reaper.SetMediaTrackInfo_Value(job.key, "B_MAINSEND", 0)
  end
end
local i, start = 1, reaper.time_precise()
local function run()
  if i > #steps then
    say("done")
    log:close()
    return
  end
  if reaper.time_precise() - start >= 1.0 then
    steps[i]()
    i = i + 1
    start = reaper.time_precise()
  end
  reaper.defer(run)
end
reaper.defer(run)
