# Offline cores

Zips of libretro cores that Vuretro installs **without internet**. Fill this folder with

```
make cores-offline          # the default core of each system
make cores-offline ARGS=--all   # also the alternatives
```

It writes `cores/offline/<platform>/<core>_libretro.<dylib|so|dll>.zip` for the OS you run it on
(copy the folder to a computer with no connection and the app finds it).

Vuretro looks, in this order, in `$VURETRO_CORES_OFFLINE`, `<data dir>/cores-offline`, and
`cores/offline` next to the app or the source it runs from. A core found there is installed
from the zip; one that is not is downloaded as usual.

The zips are not committed: they are large (the MAME core alone is ~100 MB, over what GitHub
accepts for a plain file), differ per OS and CPU, and some cores have licences that do not
allow passing them on.
