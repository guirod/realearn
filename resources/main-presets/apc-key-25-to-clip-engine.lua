-- Utility functions

--- Takes a key-value table and adds a new attribute `id` to each value that
--- corresponds to the key.
function set_keys_as_ids(t)
    for key, value in pairs(t) do
        value.id = key
    end
end

--- Puts each `label` property value of the given array into a new array.
function extract_labels(array)
    local labels = {}
    for _, element in ipairs(array) do
        table.insert(labels, element.label)
    end
    return labels
end

--- Converts the given key-value table to an array table.
function to_array(t)
    local array = {}
    for _, v in pairs(t) do
        table.insert(array, v)
    end
    return array
end

--- Returns a new table that's the given table turned into an array
--- and sorted by the `index` key.
function sorted_by_index(t)
    local sorted = to_array(t)
    local compare_index = function(left, right)
        return left.index < right.index
    end
    table.sort(sorted, compare_index)
    return sorted
end

--- Clones a table.
function clone(t)
    local new_table = {}
    for k, v in pairs(t) do
        new_table[k] = v
    end
    return new_table
end

--- Returns a new table that is the result of merging t2 into t1.
---
--- Values in t2 have precedence.
---
--- The result will be mergeable as well. This is good for "modifier chaining".
function merged(t1, t2)
    local result = clone(t1)
    for key, new_value in pairs(t2) do
        local old_value = result[key]
        if old_value and type(old_value) == "table" and type(new_value) == "table" then
            -- Merge table value as well
            result[key] = merged(old_value, new_value)
        else
            -- Simple use new value
            result[key] = new_value
        end
    end
    return make_mergeable(result)
end

--- Makes it possible to merge this table with another one via "+" operator.
function make_mergeable(t)
    local metatable = {
        __add = merged
    }
    setmetatable(t, metatable)
    return t
end

function PartialMapping(t)
    return make_mergeable(t)
end

-- Constants

local column_count = 8
local row_count = 5
local column_mode_count = 100
local knob_mode_count = 100

-- Column modes

local column_modes = {
    stop = {
        index = 0,
        label = "Stop clip",
    },
    solo = {
        index = 1,
        label = "Solo",
    },
    record_arm = {
        index = 2,
        label = "Record arm",
    },
    mute = {
        index = 3,
        label = "Mute",
    },
    select = {
        index = 4,
        label = "Track select",
    },
}
local sorted_column_modes = sorted_by_index(column_modes)

-- Knob modes
local knob_modes = {
    volume = {
        index = 0,
        label = "Volume",
    },
    pan = {
        index = 1,
        label = "Pan",
    },
    sends = {
        index = 2,
        label = "Sends",
    },
    device = {
        index = 3,
        label = "Device",
    },
}
local sorted_knob_modes = sorted_by_index(knob_modes)

-- Parameters
local params = {
    column_offset = {
        index = 0,
        name = "Column offset",
        value_count = 10000,
    },
    row_offset = {
        index = 1,
        name = "Row offset",
        value_count = 10000,
    },
    shift = {
        index = 2,
        name = "Shift modifier",
    },
    column_mode = {
        index = 3,
        name = "Column mode",
        value_count = column_mode_count,
        value_labels = extract_labels(sorted_column_modes),
    },
    knob_mode = {
        index = 4,
        name = "Knob mode",
        value_count = knob_mode_count,
        value_labels = extract_labels(sorted_knob_modes),
    },
    send = {
        index = 6,
        name = "Send",
        value_count = 2,
    },
}


-- Domain functions

function create_coordinate_expression(param, index)
    return "p[" .. param .. "] + " .. index
end

function create_col_expression(col)
    return create_coordinate_expression(params.column_offset.index, col)
end

function create_row_expression(row)
    return create_coordinate_expression(params.row_offset.index, row)
end

function create_slot_selector(col, row)
    return {
        address = "Dynamic",
        column_expression = create_col_expression(col),
        row_expression = create_row_expression(row)
    }
end

function create_column_selector(col)
    return {
        address = "Dynamic",
        expression = create_col_expression(col),
    }
end

function create_row_selector(row)
    return {
        address = "Dynamic",
        expression = create_row_expression(row),
    }
end

function multi(index)
    return PartialMapping {
        source = {
            kind = "Virtual",
            character = "Multi",
            id = index,
        },
    }
end

function button(id)
    return PartialMapping {
        source = {
            kind = "Virtual",
            id = id,
            character = "Button",
        },
    }
end

function column_stop_button(col)
    return button("col" .. (col + 1) .. "/stop")
end

function row_play_button(row)
    return button("row" .. (row + 1) .. "/play")
end

function slot_button(col, row)
    return button("col" .. (col + 1) .. "/row" .. (row + 1) .. "/pad")
end

function shift_pressed(on)
    return PartialMapping {
        activation_condition = {
            kind = "Modifier",
            modifiers = {
                {
                    parameter = params.shift.index,
                    on = on,
                },
            },
        },
    }
end

function fire_after_timeout(millis)
    return PartialMapping {
        glue = {
            fire_mode = {
                kind = "AfterTimeout",
                timeout = millis,
            },
        },
    }
end

function fire(kind)
    return {
        glue = {
            fire_mode = {
                kind = kind,
            },
        },
    }
end

local shift = shift_pressed(true)
local no_shift = shift_pressed(false)
local long_press = fire_after_timeout(1000)
local single_press = fire("OnSinglePress")
local double_press = fire("OnDoublePress")

local device_specific = {
    apc_key_25 = {
        volume = shift + button("col5/stop"),
        pan = shift + button("col6/stop"),
        device = shift + button("col8/stop"),
        solo = shift + button("row2/play"),
        record_arm = shift + button("row3/play"),
        mute = shift + button("row4/play"),
        track_select = shift + button("row5/play"),
        slot_normal_condition = no_shift,
        slot_quantize_condition = shift + double_press,
        rec = no_shift + button("record"),
    },
}

function clip_matrix_action(action)
    return PartialMapping {
        target = {
            kind = "ClipMatrixAction",
            action = action,
        },
    }
end

function clip_column_action(col, action)
    return PartialMapping {
        target = {
            kind = "ClipColumnAction",
            column = create_column_selector(col),
            action = action,
        },
    }
end

function clip_row_action(row, action)
    return PartialMapping {
        target = {
            kind = "ClipRowAction",
            row = create_row_selector(row),
            action = action,
        },
    }
end

function clip_column_track(col)
    return {
        address = "FromClipColumn",
        column = create_column_selector(col),
        context = "Playback",
    }
end

function column_track_target(col, track_target_kind, exclusive)
    return PartialMapping {
        target = {
            kind = track_target_kind,
            track = clip_column_track(col),
            exclusivity = exclusive and "WithinFolderOnOnly",
        },
    }
end

function route_target(col, route_target_kind)
    return PartialMapping {
        target = {
            kind = route_target_kind,
            route = {
                address = "Dynamic",
                track = clip_column_track(col),
                expression = "p[" .. params.send.index .. "]",
            },
        },
    }
end

function clip_transport_action(col, row, action, record_only_if_track_armed)
    return PartialMapping {
        target = {
            kind = "ClipTransportAction",
            slot = create_slot_selector(col, row),
            action = action,
            record_only_if_track_armed = record_only_if_track_armed,
            stop_column_if_slot_empty = true,
        },
    }
end

function clip_management_action(col, row, action)
    return PartialMapping {
        target = {
            kind = "ClipManagement",
            slot = create_slot_selector(col, row),
            action = {
                kind = action,
            },
        },
    }
end

function slot_state_text_feedback()
    return PartialMapping {
        glue = {
            feedback = {
                kind = "Text",
                text_expression = "{{ target.slot_state.id }}",
            },
        },
    }
end

function transport_action(action)
    return PartialMapping {
        target = {
            kind = "TransportAction",
            action = action,
        },
    }
end

function reaper_action(command_id)
    return PartialMapping {
        target = {
            kind = "ReaperAction",
            command = command_id,
            invocation = "Trigger",
        },
    }
end

function toggle()
    return PartialMapping {
        glue = {
            absolute_mode = "ToggleButton",
        },
    }
end

function incremental()
    return PartialMapping {
        glue = {
            absolute_mode = "IncrementalButton",
        },
    }
end

function wrap()
    return PartialMapping {
        glue = {
            wrap = true,
        },
    }
end

function control_disabled()
    return PartialMapping {
        control_enabled = false,
    }
end

function feedback_disabled()
    return PartialMapping {
        feedback_enabled = false,
    }
end

function scroll_horizontally(amount)
    return scroll(params.column_offset.index, amount)
end

function scroll_vertically(amount)
    return scroll(params.row_offset.index, amount)
end

function scroll(param_index, amount)
    local abs_amount = math.abs(amount)
    return {
        glue = {
            absolute_mode = "IncrementalButton",
            step_factor_interval = { abs_amount, abs_amount },
            reverse = amount < 0,
            feedback = {
                kind = "Numeric",
                transformation = "x = 1",
            },
        },
        target = {
            kind = "FxParameterValue",
            parameter = {
                address = "ById",
                index = param_index,
            },
        },
    }
end

function set_param(index)
    return PartialMapping {
        target = {
            kind = "FxParameterValue",
            parameter = {
                address = "ById",
                index = index,
            },
        },
    }
end

function group(g)
    return PartialMapping {
        group = g.id,
    }
end

function set_mode(mode, mode_count, mode_param_index)
    local target_value = mode.index / (mode_count - 1)
    return PartialMapping {
        name = mode.label,
        glue = {
            target_interval = { target_value, target_value },
            out_of_range_behavior = "Min",
        },
        target = {
            kind = "FxParameterValue",
            parameter = {
                address = "ById",
                index = mode_param_index,
            },
        },
    }
end

function set_column_mode(mode)
    return set_mode(mode, column_mode_count, params.column_mode.index)
end

function set_knob_mode(mode)
    return set_mode(mode, knob_mode_count, params.knob_mode.index)
end

function column_mode_is(column_mode)
    return {
        kind = "Bank",
        parameter = params.column_mode.index,
        bank_index = column_mode.index,
    }
end

function knob_mode_is(knob_mode)
    return {
        kind = "Bank",
        parameter = params.knob_mode.index,
        bank_index = knob_mode.index,
    }
end

-- Groups

local groups = {
    slot_modes = {
        name = "Slot modes",
    },
    column_modes = {
        name = "Column modes",
    },
    knob_modes = {
        name = "Knob modes",
    },
    slot_feedback = {
        name = "Slot feedback",
    },
    slot_play = {
        name = "Slot play",
    },
    slot_clear = {
        name = "Slot clear",
    },
    slot_quantize = {
        name = "Slot quantize",
    },
    column_stop = {
        name = "Column stop",
        activation_condition = column_mode_is(column_modes.stop),
    },
    row_play = {
        name = "Row play",
    },
    column_solo = {
        name = "Column solo",
        activation_condition = column_mode_is(column_modes.solo),
    },
    column_record_arm = {
        name = "Column record arm",
        activation_condition = column_mode_is(column_modes.record_arm),
    },
    column_mute = {
        name = "Column mute",
        activation_condition = column_mode_is(column_modes.mute),
    },
    column_select = {
        name = "Column select",
        activation_condition = column_mode_is(column_modes.select),
    },
    knob_volume = {
        name = "Knob volume",
        activation_condition = knob_mode_is(knob_modes.volume),
    },
    knob_pan = {
        name = "Knob pan",
        activation_condition = knob_mode_is(knob_modes.pan),
    },
    knob_sends = {
        name = "Knob sends",
        activation_condition = knob_mode_is(knob_modes.sends),
    },
    knob_device = {
        name = "Knob device",
        activation_condition = knob_mode_is(knob_modes.device),
    },
}
set_keys_as_ids(groups)

-- Mappings

local mappings = {
    no_shift + button("stop-all-clips") + clip_matrix_action("Stop"),
    no_shift + button("play") + toggle() + transport_action("PlayStop"),
    shift + button("col1/stop") + feedback_disabled() + scroll_vertically(-1),
    shift + button("col2/stop") + feedback_disabled() + scroll_vertically(1),
    shift + button("col3/stop") + feedback_disabled() + scroll_horizontally(-1),
    shift + button("col4/stop") + feedback_disabled() + scroll_horizontally(1),
    button("shift") + set_param(params.shift.index),
    shift + button("play") + clip_matrix_action("Undo"),
    shift + button("record") + clip_matrix_action("Redo"),
    shift + button("stop-all-clips") + reaper_action(40364),
    group(groups.knob_sends) + feedback_disabled() + shift + button("col7/stop") + incremental() + wrap() + set_param(params.send.index),
    group(groups.column_modes) + shift + button("row1/play") + set_column_mode(column_modes.stop),
    group(groups.column_modes) + shift + button("row2/play") + set_column_mode(column_modes.solo),
    group(groups.column_modes) + shift + button("row3/play") + set_column_mode(column_modes.record_arm),
    group(groups.column_modes) + shift + button("row4/play") + set_column_mode(column_modes.mute),
    group(groups.column_modes) + shift + button("row5/play") + set_column_mode(column_modes.select),
    group(groups.knob_modes) + shift + button("col5/stop") + set_knob_mode(knob_modes.volume),
    group(groups.knob_modes) + shift + button("col6/stop") + set_knob_mode(knob_modes.pan),
    group(groups.knob_modes) + shift + button("col7/stop") + set_knob_mode(knob_modes.sends),
    group(groups.knob_modes) + shift + button("col8/stop") + set_knob_mode(knob_modes.device),
}

-- For each column
for col = 0, column_count - 1 do
    -- Column stop button functions
    table.insert(mappings, group(groups.column_stop) + no_shift + column_stop_button(col) + clip_column_action(col, "Stop"))
    table.insert(mappings, group(groups.column_solo) + no_shift + toggle() + column_stop_button(col) + column_track_target(col, "TrackSoloState"))
    table.insert(mappings, group(groups.column_record_arm) + no_shift + toggle() + column_stop_button(col) + column_track_target(col, "TrackArmState", true))
    table.insert(mappings, group(groups.column_mute) + no_shift + toggle() + column_stop_button(col) + column_track_target(col, "TrackMuteState"))
    table.insert(mappings, group(groups.column_select) + no_shift + toggle() + column_stop_button(col) + column_track_target(col, "TrackSelectionState", true))
    -- Knob functions
    table.insert(mappings, group(groups.knob_volume) + multi(col) + column_track_target(col, "TrackVolume"))
    table.insert(mappings, group(groups.knob_pan) + multi(col) + column_track_target(col, "TrackPan"))
    table.insert(mappings, group(groups.knob_sends) + multi(col) + route_target(col, "RouteVolume"))
end

-- For each row
for row = 0, row_count - 1 do
    table.insert(mappings, group(groups.row_play) + feedback_disabled() + no_shift + row_play_button(row) + clip_row_action(row, "Play"))
end

-- For each slot
for col = 0, column_count - 1 do
    for row = 0, row_count - 1 do
        table.insert(mappings, group(groups.slot_play) + feedback_disabled() + no_shift + slot_button(col, row) + toggle() + clip_transport_action(col, row, "RecordPlayStop", true))
        --table.insert(mappings, group(groups.slot_play) + feedback_disabled() + shift + single_press + slot_button(col, row) + toggle() + clip_transport_action(col, row, "RecordStop", false))
        table.insert(mappings, group(groups.slot_feedback) + control_disabled() + slot_button(col, row) + slot_state_text_feedback() + clip_transport_action(col, row, "RecordPlayStop", true))
        table.insert(mappings, group(groups.slot_clear) + feedback_disabled() + shift + long_press + slot_button(col, row) + clip_management_action(col, row, "ClearSlot"))
        table.insert(mappings, group(groups.slot_quantize) + feedback_disabled() + shift + double_press + slot_button(col, row) + toggle() + clip_management_action(col, row, "EditClip"))
    end
end

return {
    kind = "MainCompartment",
    value = {
        parameters = sorted_by_index(params),
        groups = to_array(groups),
        mappings = mappings,
    },
}