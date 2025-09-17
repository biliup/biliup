import stream_gears

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib

config = stream_gears.config_bindings()