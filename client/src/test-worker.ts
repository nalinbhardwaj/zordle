import { expose } from 'comlink';

async function get_play_diff() {
    console.log('diffing');
    const multiThread = await import('halowordle');
    await multiThread.default();
    await multiThread.initThreadPool(navigator.hardwareConcurrency);
    multiThread.init_panic_hook();
    const ret = multiThread.get_play_diff("fluff", ["fluff", "fluff", "fluff", "fluff", "fluff", "fluff"]);
    return ret;
}

// async function write_params() {
//     console.log('genning');
//     const multiThread = await import('halowordle');
//     await multiThread.default();
//     await multiThread.initThreadPool(navigator.hardwareConcurrency);
//     multiThread.init_panic_hook();
//     console.log('here we go');
//     const ret = multiThread.write_params();
//     return ret;
// }

// async function verify_play() {
//     console.log('genning');
//     const multiThread = await import(
//         'halowordle'
//       );
//     await multiThread.default();
//     await multiThread.initThreadPool(navigator.hardwareConcurrency);
//     console.log('here we go');
//     const ret = multiThread.verify_play();
//     return ret;
// }

async function prove_play() {
    const response = await fetch('http://localhost:3000/params.bin');
    const bytes = await response.arrayBuffer();
    const params = new Uint8Array(bytes);
    console.log("param length", params.length);
    console.log("params", params);

    console.log('genning proof');
    const multiThread = await import(
        'halowordle'
      );
    await multiThread.default();
    await multiThread.initThreadPool(navigator.hardwareConcurrency);
    console.log('here we go');
    const ret = multiThread.prove_play("fluff",
    ["fluff", "fluff", "fluff", "fluff", "fluff", "fluff"], params);
    return ret;
}

const exports = {
    get_play_diff,
    prove_play
};
export type MyFirstWorker = typeof exports;

expose(exports);