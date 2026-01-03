--@module = true

local mod = reqscript('internal/mod')

local scriptmanager = require("script-manager")

-- setup the MOD
mod.p("is loading v" .. mod.DISPLAYED_VERSION .. "...")
mod.setup()

-- load translation data
mod.load()

-- setup hooks
mod.p("is initializing...")
mod.init()

-- enable hooks
mod.p("is enabling...")
mod.enable()

-- setup hooks on DFHack
mod.setup_hooks()

-- done
mod.p("has been enabled.")
