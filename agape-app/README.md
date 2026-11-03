# agape-app

Screens over the Agape kernel. `node build.mjs` embeds the kernel's WASM build
(`cargo build --profile wasm --target wasm32-unknown-unknown --no-default-features` in ../agape-kernel)
and the venue and event templates into one page, `dist/agape-live.html`.

- `src/kernel.ts`  load the kernel, `view()` a lens, `act()` an action
- `src/parts.tsx`  the seven shared components
- `src/fan.tsx`    the fan's lenses (Now, Map, Crowd, Me) and the order and checkout sheets
- `src/roles.tsx`  runner, stand and operations lenses
- `src/main.tsx`   scoreboard, clock, perspective switch
