Changes in the code for better readability are mentioned below:

1. [ ] Reevaluate scope of functions and arguments to privatise and manage scope
       of whatever that is not required in public APIs.
       It is important for providing a clear boundary for events/structs that
       need to be where. It involves following:
   - Privatise `WindowConf` fields, move docs to `WindowConfBuilder`.
   - Remove `layer_properties` and make `configure` public.
   - Add comments in order of initialisation of Wayland objects.
2. [ ] Break out large initialisation functions of `SpellWin` and `SpellLock` into
       helper functions. Separate the implementations that are internal and that
       are public facing.
3. [ ] Properties that are implemented for `SpellWin` but not for `SpellLock`.
   - Fractional scaling.
   - Macro creation for helper methods.
4. [ ] Remover bin binaries from everywhere except the demo.
5. [ ] remove the match matching in CLI and uses clap for better management.
