// Browser proof for the web build: draws, reloads, downloads, opens, drops and switches tabs
// through real input, and writes screenshots and frame timings to the output folder.
//
//   node app/rusty-pencil/web/smoke.mjs [target/web] [target/web-proof]
//
// Needs Playwright (PLAYWRIGHT_MODULE overrides the import) and a Chromium with WebGPU
// (CHROMIUM_PATH). Headless Chromium on Linux renders WebGPU but captures it as transparent, so
// HEADED=1 runs a visible window on the current display for real screenshots.
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { extname, resolve, sep } from 'node:path';

const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = resolve(process.argv[2] || 'target/web');
const output = resolve(process.argv[3] || 'target/web-proof');
const fixtures = resolve(import.meta.dirname, '../../../crates/pen-core/tests/fixtures');
const fixture = `${fixtures}/landscape.png`;
await mkdir(output, { recursive: true });

const types = { '.wasm': 'application/wasm', '.js': 'text/javascript', '.html': 'text/html', '.svg': 'image/svg+xml' };
const server = createServer(async (request, response) => {
    const path = resolve(root, '.' + new URL(request.url, 'http://localhost').pathname.replace(/\/$/, '/index.html'));
    if (!path.startsWith(root + sep)) { response.writeHead(403).end(); return; }
    try {
        const bytes = await readFile(path);
        response.setHeader('Content-Type', types[extname(path)] || 'application/octet-stream');
        response.end(bytes);
    } catch { response.writeHead(404).end(); }
});
await new Promise(done => server.listen(0, '127.0.0.1', done));
const url = `http://127.0.0.1:${server.address().port}`;

const browser = await chromium.launch({
    headless: !process.env.HEADED,
    executablePath: process.env.CHROMIUM_PATH,
    args: ['--enable-unsafe-webgpu', '--enable-features=Vulkan',
        ...(process.env.WAYLAND_DISPLAY ? ['--ozone-platform=wayland'] : [])],
});
const errors = [];
const results = {};
try {
    const context = await browser.newContext({ viewport: { width: 1280, height: 800 }, acceptDownloads: true,
        permissions: ['clipboard-read', 'clipboard-write'] });
    const page = await context.newPage();
    page.on('pageerror', e => errors.push(e.message));
    page.on('console', m => { if (m.type() === 'error' && !m.text().includes('404')) errors.push(m.text()); });
    await page.addInitScript(() => {
        window.__proof = { submits: 0, frames: [] };
        if (!globalThis.GPUQueue) return;
        const submit = GPUQueue.prototype.submit;
        GPUQueue.prototype.submit = function (...args) {
            window.__proof.submits++;
            window.__proof.frames.push(performance.now());
            return submit.apply(this, args);
        };
    });

    // The canvas through WebGPU, as the page shows it (the page screenshot when headed).
    const shot = async name => {
        await page.waitForTimeout(400);
        await page.screenshot({ path: `${output}/${name}.png` });
    };
    const ready = async () => {
        await page.waitForFunction(() => window.__proof?.submits > 0, null, { timeout: 30000 });
        await page.waitForTimeout(800);
    };
    // Stored app data, parsed: IndexedDB `rusty-pencil` / `data`, keyed by path.
    const stored = () => page.evaluate(() => new Promise((ok, fail) => {
        const open = indexedDB.open('rusty-pencil', 1);
        open.onerror = () => fail(open.error);
        open.onsuccess = () => {
            const store = open.result.transaction('data').objectStore('data');
            const keys = store.getAllKeys(), values = store.getAll();
            values.onsuccess = () => {
                const text = new TextDecoder();
                ok(Object.fromEntries(keys.result.map((k, i) => [k, text.decode(values.result[i])])));
                open.result.close();
            };
        };
    }));
    const elements = async () => {
        const data = await stored();
        const workspace = JSON.parse(data['/rusty-pencil/workspace.json'] ?? 'null');
        const active = workspace?.tabs.find(t => t.id === workspace.active);
        return JSON.parse(data[active?.recovery ?? '/rusty-pencil/autosave.drusty-pen'] ?? '{"elements":[]}').elements.length;
    };
    const drag = async (from, to, steps = 24) => {
        await page.mouse.move(...from);
        await page.mouse.down();
        await page.mouse.move(...to, { steps });
        await page.mouse.up();
    };

    await page.goto(url);
    await ready();
    await shot('1-empty');

    // Draw: frame intervals while the pointer moves, as a smoothness check.
    await page.keyboard.press('2');
    await page.evaluate(() => { window.__proof.frames = []; });
    await drag([480, 300], [720, 470], 60);
    const frames = await page.evaluate(() => window.__proof.frames);
    const gaps = frames.slice(1).map((t, i) => t - frames[i]).sort((a, b) => a - b);
    results.drawFrames = { count: frames.length, medianMs: gaps[gaps.length >> 1], p95Ms: gaps[Math.floor(gaps.length * 0.95)] };
    await page.keyboard.press('4');
    await drag([800, 320], [980, 460]);
    await page.keyboard.press('Escape');
    await shot('2-drawn');

    // Autosave and workspace survive a reload after the one-second debounce.
    await page.waitForTimeout(1500);
    assert.equal(await elements(), 2, 'autosave stored both shapes');
    await page.reload();
    await ready();
    await shot('3-restored');

    // Hiding the page flushes an edit made moments before the reload.
    await page.keyboard.press('2');
    await drag([500, 560], [640, 650]);
    await page.keyboard.press('Escape');
    await page.reload();
    await ready();
    results.flushedOnReload = await elements();
    assert.equal(results.flushedOnReload, 3, 'pagehide flushed the last edit');

    // Preferences persist: dark mode stays on across a reload.
    await page.keyboard.press('Alt+Shift+D');
    await page.waitForTimeout(300);
    assert.equal(JSON.parse((await stored())['/rusty-pencil/preferences.json']).dark, true);
    await page.reload();
    await ready();
    await shot('4-dark-restored');
    await page.keyboard.press('Alt+Shift+D');

    // Save downloads the scene.
    const [download] = await Promise.all([page.waitForEvent('download'), page.keyboard.press('Control+s')]);
    const saved = `${output}/${download.suggestedFilename()}`;
    await download.saveAs(saved);
    results.saved = download.suggestedFilename();
    assert.equal(JSON.parse(await readFile(saved, 'utf8')).elements.length, 3);

    // A new tab (Alt+T on web), then open the saved scene through the file picker.
    await page.keyboard.press('Alt+t');
    const [chooser] = await Promise.all([page.waitForEvent('filechooser'), page.keyboard.press('Control+o')]);
    await chooser.setFiles(saved);
    await page.waitForTimeout(500);
    await shot('5-opened');

    // Drop an image and a scene onto the canvas.
    const image = (await readFile(fixture)).toString('base64');
    await page.evaluate(async ([png]) => {
        const bytes = Uint8Array.from(atob(png), c => c.charCodeAt(0));
        const transfer = new DataTransfer();
        transfer.items.add(new File([bytes], 'landscape.png', { type: 'image/png' }));
        const at = { clientX: 300, clientY: 520, dataTransfer: transfer, bubbles: true, cancelable: true };
        document.querySelector('canvas').dispatchEvent(new DragEvent('dragover', at));
        document.querySelector('canvas').dispatchEvent(new DragEvent('drop', at));
    }, [image]);
    await page.waitForTimeout(600);
    await shot('6-dropped');
    await page.waitForTimeout(1300);
    assert.equal(await elements(), 4, 'the dropped image joined the opened scene');

    // Copy and paste through the browser clipboard.
    await page.keyboard.press('Control+a');
    await page.keyboard.press('Control+c');
    await page.waitForTimeout(300);
    await page.keyboard.press('Alt+t');
    await page.waitForTimeout(300);
    await page.keyboard.press('Control+v');
    await page.waitForTimeout(1600);
    assert.equal(await elements(), 4, 'paste copied all four elements into the new tab');
    await shot('7-pasted');

    // Plain text from another app pastes as a text element.
    await page.evaluate(() => navigator.clipboard.writeText('Hello from the clipboard'));
    await page.keyboard.press('Escape');
    await page.keyboard.press('Control+v');
    await page.waitForTimeout(1600);
    assert.equal(await elements(), 5, 'plain text pasted as one more element');

    // Export PNG and SVG as downloads from the export dialog.
    const save = async (x, y) => {
        await page.keyboard.press('Control+Shift+E');
        await page.waitForTimeout(400);
        const [file] = await Promise.all([page.waitForEvent('download'), page.mouse.click(x, y)]);
        await file.saveAs(`${output}/${file.suggestedFilename()}`);
        return file.suggestedFilename();
    };
    results.exports = [await save(516, 454), await save(604, 454)];
    assert.deepEqual(results.exports, ['Drawing.png', 'Drawing.svg']);
    const png = await readFile(`${output}/Drawing.png`);
    assert.equal(png.subarray(1, 4).toString(), 'PNG');
    assert.match(await readFile(`${output}/Drawing.svg`, 'utf8'), /<svg/);

    // The personal library: add the selection, export it, and survive a reload.
    await page.keyboard.press('Control+a');
    await page.mouse.click(1143, 240);
    await page.waitForTimeout(300);
    await page.mouse.click(1143, 359);
    await page.waitForTimeout(300);
    await shot('9-library');
    const [library] = await Promise.all([page.waitForEvent('download'), page.mouse.click(1124, 403)]);
    results.library = library.suggestedFilename();
    await library.saveAs(`${output}/${results.library}`);
    assert.equal(JSON.parse(await readFile(`${output}/${results.library}`, 'utf8')).items.length, 1);
    const libraryItems = async () => JSON.parse((await stored())['/rusty-pencil/library.drusty-library']).items.length;
    assert.equal(await libraryItems(), 1, 'library stored');
    // Remove the item, then import it back from the exported file.
    await page.mouse.click(1143, 559);
    await page.waitForTimeout(300);
    assert.equal(await libraryItems(), 0);
    const [importer] = await Promise.all([page.waitForEvent('filechooser'), page.mouse.click(1062, 403)]);
    await importer.setFiles(`${output}/${results.library}`);
    await page.waitForTimeout(600);
    assert.equal(await libraryItems(), 1, 'library imported from file');
    await page.mouse.click(1143, 240);

    // An image on the clipboard pastes as an image element.
    await page.keyboard.press('Escape');
    await page.evaluate(async ([image]) => {
        const bytes = Uint8Array.from(atob(image), c => c.charCodeAt(0));
        await navigator.clipboard.write([new ClipboardItem({ 'image/png': new Blob([bytes], { type: 'image/png' }) })]);
    }, [image]);
    await page.keyboard.press('Control+v');
    await page.waitForTimeout(1600);
    assert.equal(await elements(), 6, 'clipboard image pasted');

    // An Excalidraw file opens through the picker.
    const [excalidraw] = await Promise.all([page.waitForEvent('filechooser'), page.keyboard.press('Control+o')]);
    await excalidraw.setFiles(`${fixtures}/all-elements.excalidraw`);
    await page.waitForTimeout(800);
    await shot('10-excalidraw');

    // Pan and zoom: frame intervals while the wheel scrolls.
    await page.mouse.move(640, 400);
    await page.evaluate(() => { window.__proof.frames = []; });
    for (let i = 0; i < 30; i++) { await page.mouse.wheel(0, 20); await page.waitForTimeout(16); }
    const pan = await page.evaluate(() => window.__proof.frames);
    const panGaps = pan.slice(1).map((t, i) => t - pan[i]).sort((a, b) => a - b);
    results.panFrames = { count: pan.length, medianMs: panGaps[panGaps.length >> 1], p95Ms: panGaps[Math.floor(panGaps.length * 0.95)] };
    await shot('8-panned');

    assert.deepEqual(errors, []);
    await writeFile(`${output}/results.json`, JSON.stringify(results, null, 2));
    console.log(JSON.stringify(results));
} finally {
    if (errors.length) console.log('page errors:', errors);
    await browser.close();
    server.close();
}
