-- Constants

local channel_count = 8

-- Utility functions

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
}

-- Domain functions

function current_slot(channel_index)
    return {
        address = "Dynamic",
        column_expression = "p[0] + " .. channel_index,
        row_expression = "p[1]"
    }
end

function current_column(channel_index)
    return {
        address = "Dynamic",
        expression = "p[0] + " .. channel_index,
    }
end

function button(id)
    return PartialMapping {
        source = {
            kind = "Virtual",
            character = "Button",
            id = id,
        },
    }
end

function multi(id)
    return PartialMapping {
        source = {
            kind = "Virtual",
            character = "Multi",
            id = id,
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

function clip_transport_action(action, channel_index)
    return PartialMapping {
        target = {
            kind = "ClipTransportAction",
            slot = current_slot(channel_index),
            action = action,
            record_only_if_track_armed = true,
            stop_column_if_slot_empty = true,
        },
    }
end

function clip_name_feedback(channel_index)
    return PartialMapping {
        control_enabled = false,
        glue = {
            feedback = {
                kind = "Text",
                text_expression = "{{ target.clip.name }}"
            },
        },
        target = {
            kind = "ClipManagement",
            slot = current_slot(channel_index),
            action = {
                kind = "EditClip",
            },
        },
    }
end

function clip_position_feedback(channel_index)
    return PartialMapping {
        control_enabled = false,
        target = {
            kind = "ClipSeek",
            slot = current_slot(channel_index),
            -- TODO-high-playtime-after-release The Platform M+ seems to don't like resolution High (skips messages). Maybe time for #533.
            feedback_resolution = "Beat",
        },
    }
end

function clip_volume(channel_index)
    return {
        target = {
            kind = "ClipVolume",
            slot = current_slot(channel_index),
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

function channel_button(channel_index, id)
    return button("ch" .. (channel_index + 1) .. "/" .. id)
end

function channel_multi(channel_index, id)
    return multi("ch" .. (channel_index + 1) .. "/" .. id)
end

function control_off()
    return PartialMapping {
        control_enabled = false,
    }
end

function feedback_off()
    return PartialMapping {
        feedback_enabled = false,
    }
end

function column_track_target(channel_index, target_kind, exclusive)
    return {
        glue = {
            absolute_mode = "ToggleButton",
        },
        target = {
            kind = target_kind,
            track = {
                address = "FromClipColumn",
                column = current_column(channel_index),
                context = "Playback",
            },
            exclusivity = exclusive and "WithinFolderOnOnly",
        },
    }
end

-- Mappings

local mappings = {
    button("play") + toggle() + transport_action("PlayPause"),
    button("stop") + transport_action("Stop"),
    button("cursor-left") + scroll_horizontally(-1),
    button("cursor-right")+ scroll_horizontally(1),
    button("cursor-up") + scroll_vertically(-1),
    button("cursor-down") + scroll_vertically(1),
}

for ch = 0, channel_count - 1 do
    table.insert(mappings, channel_button(ch, "v-select") + toggle() + feedback_off() + clip_transport_action("RecordPlayStop", ch))
    table.insert(mappings, channel_button(ch, "select") + toggle() + control_off() + clip_transport_action("RecordPlayStop", ch))
    table.insert(mappings, channel_multi(ch, "fader") + clip_position_feedback(ch))
    table.insert(mappings, channel_multi(ch, "v-pot") + clip_volume(ch))
    table.insert(mappings, channel_button(ch, "mute") + column_track_target(ch, "TrackMuteState"))
    table.insert(mappings, channel_button(ch, "solo") + column_track_target(ch, "TrackSoloState"))
    table.insert(mappings, channel_button(ch, "record-ready") + column_track_target(ch, "TrackArmState", true))
end

-- Result

return {
    kind = "MainCompartment",
    value = {
        parameters = sorted_by_index(params),
        mappings = mappings,
    },
}