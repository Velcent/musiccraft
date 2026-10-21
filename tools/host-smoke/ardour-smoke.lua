-- MusicCraft VST3 plugins in Ardour, headless (ardour8-lua, dummy backend).
local dir = os.getenv("MC_DIR")
AudioEngine:set_backend("None (Dummy)", "", "")
AudioEngine:set_sample_rate(48000)
AudioEngine:start(false)

local function note(track, status, key, vel)
  local buf = C.ByteVector()
  buf:add({ status, key, vel })
  assert(track:write_immediate_event(Evoral.EventType.MIDI_EVENT, 3, buf:to_array()))
end

-- The master bus's highest peak since the last reset, in dB.
local function peak(s)
  return s:master_out():peak_meter():meter_level(0, ARDOUR.MeterType.MeterMaxPeak)
end

local s = create_session(dir, "smoke", 48000)
assert(s, "no session")
for _, inst in ipairs({ "Synth", "Antiserum" }) do
  local info = ARDOUR.LuaAPI.new_plugin_info(inst, ARDOUR.PluginType.VST3)
  assert(not info:isnil(), inst .. " not found")
  local tracks = s:new_midi_track(
    ARDOUR.ChanCount(ARDOUR.DataType("midi"), 1),
    ARDOUR.ChanCount(ARDOUR.DataType("audio"), 2),
    true, info, nil, nil, 1, inst, ARDOUR.PresentationInfo.max_order,
    ARDOUR.TrackMode.Normal, true, false)
  local track = tracks:front()
  local delay = ARDOUR.LuaAPI.new_plugin(s, "Delay", ARDOUR.PluginType.VST3, "")
  assert(not delay:isnil(), "Delay not found")
  track:add_processor_by_index(delay, -1, nil, true)
  -- Delay's mix (parameter 3) to 0.6 through Ardour's control.
  local ok = ARDOUR.LuaAPI.set_processor_param(delay, 3, 0.6)
  print(inst, "set delay param", ok, "now", ARDOUR.LuaAPI.get_processor_param(delay, 3, true))
  s:request_roll(ARDOUR.TransportRequestSource.TRS_UI)
  ARDOUR.LuaAPI.usleep(300000)
  s:master_out():peak_meter():reset_max()
  ARDOUR.LuaAPI.usleep(300000)
  local quiet = peak(s)
  for _, key in ipairs({ 60, 64, 67 }) do note(track, 0x90, key, 100) end
  ARDOUR.LuaAPI.usleep(1000000)
  local loudest = peak(s)
  for _, key in ipairs({ 60, 64, 67 }) do note(track, 0x80, key, 0) end
  ARDOUR.LuaAPI.usleep(1500000)
  s:request_stop(false, false, ARDOUR.TransportRequestSource.TRS_UI)
  print(inst, "master peak before notes", quiet, "dB, while playing", loudest, "dB")
end
s:save_state("", false, false, false, false, false)
close_session()
print("saved and closed")

local s2 = load_session(dir, "smoke")
assert(s2, "no session on reopen")
for r in s2:get_routes():iter() do
  local names = {}
  local i = 0
  while true do
    local p = r:nth_plugin(i)
    if p:isnil() then break end
    names[#names + 1] = p:display_name()
    i = i + 1
  end
  if #names > 0 then
    local delay = r:nth_plugin(#names - 1)
    print("reopened", r:name(), table.concat(names, ", "),
      "delay param", ARDOUR.LuaAPI.get_processor_param(delay, 3, true))
  end
end
close_session()
print("done")
