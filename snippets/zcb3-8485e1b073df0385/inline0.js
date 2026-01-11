
export async function bundle_and_download() {
    console.log("bundling html...");
    try {
        // get current html
        const htmlReq = await fetch(window.location.href);
        const htmlText = await htmlReq.text();
        const parser = new DOMParser();
        const doc = parser.parseFromString(htmlText, "text/html");

        // find scripts/links
        const wasmLink = doc.querySelector('link[rel="preload"][as="fetch"][type="application/wasm"]');
        const jsLink = doc.querySelector('link[rel="modulepreload"]');

        if (!wasmLink || !jsLink) {
            throw new Error("Could not find WASM or JS links in index.html");
        }

        const wasmUrl = wasmLink.href;
        const jsUrl = jsLink.href;

        // fetch assets
        console.log("fetching assets...");
        const [wasmResp, jsResp] = await Promise.all([fetch(wasmUrl), fetch(jsUrl)]);
        
        if (!wasmResp.ok || !jsResp.ok) {
            throw new Error(`Failed to fetch assets. WASM: ${wasmResp.status}, JS: ${jsResp.status}`);
        }

        let jsText = await jsResp.text();

        // inline snippets (wasm-bindgen externals)
        console.log("inlining snippets...");
        const snippetRegex = /import\s*\{([^}]+)\}\s*from\s*['"](\.\/snippets\/[^'"]+)['"];/g;
        let match;
        const replacements = [];
        
        // find all matches first
        while ((match = snippetRegex.exec(jsText)) !== null) {
            replacements.push({
                fullMatch: match[0],
                importNames: match[1],
                relativePath: match[2]
            });
        }

        // process replacements
        for (const r of replacements) {
            try {
                // construct absolute URL for the snippet
                // jsUrl is like http://.../dist/zcb3-xxxx.js
                // relativePath is like ./snippets/...
                const snippetUrl = new URL(r.relativePath, jsUrl).href;
                console.log(`fetching snippet: ${snippetUrl}`);
                
                const snippetResp = await fetch(snippetUrl);
                if (!snippetResp.ok) throw new Error(`Failed to fetch snippet ${snippetUrl}`);
                
                const snippetText = await snippetResp.text();
                const snippetBlob = new Blob([snippetText], {type: "application/javascript"});
                const reader = new FileReader();
                const snippetDataUrl = await new Promise((resolve) => {
                    reader.onloadend = () => resolve(reader.result);
                    reader.readAsDataURL(snippetBlob);
                });
                
                // replace in js
                const newLine = `import {${r.importNames}} from '${snippetDataUrl}';`;
                jsText = jsText.replace(r.fullMatch, newLine)                
            } catch (e) {
                console.warn("failed to inline snippet:", r.relativePath, e);
            }
        }

        // encode wasm
        console.log("encoding wasm...");
        const wasmBlob = await wasmResp.blob();
        const reader = new FileReader();
        const wasmBase64 = await new Promise((resolve, reject) => {
            reader.onload = () => {
                const result = reader.result;
                // result is "data:application/wasm;base64,....."
                const base64 = result.split(',')[1];
                resolve(base64);
            };
            reader.onerror = reject;
            reader.readAsDataURL(wasmBlob);
        });

        // clean up html
        wasmLink.remove();
        doc.querySelectorAll('link[rel="modulepreload"]').forEach(el => el.remove());
        
        // remove trunk's script
        const trunkScript = doc.querySelector('script[type="module"]');
        if (trunkScript && trunkScript.textContent.includes('init')) {
            trunkScript.remove();
        }
        
        // remove the trunk reload script
        const allScripts = doc.querySelectorAll('script');
        for (const s of allScripts) {
            if (s.textContent.includes('__TRUNK_ADDRESS__')) {
                s.remove();
            }
        }

        // inline favicons
        console.log("inlining favicons...");
        const icons = doc.querySelectorAll('link[rel="icon"], link[rel="shortcut icon"], link[rel="apple-touch-icon"]');
        for (const icon of icons) {
            if (icon.href) {
                try {
                    const resp = await fetch(icon.href);
                    if (resp.ok) {
                        const blob = await resp.blob();
                        const reader = new FileReader();
                        await new Promise((resolve) => {
                            reader.onloadend = resolve;
                            reader.readAsDataURL(blob);
                        });
                        icon.href = reader.result;
                    }
                } catch (e) {
                    console.warn("failed to bundle icon:", icon.href, e);
                }
            }
        }

        // inject new loader script
        console.log("injecting loader...");
        const loaderScript = doc.createElement("script");
        loaderScript.type = "module";
        
        loaderScript.textContent = `
(async function() {
    try {
        const wasmBase64 = "${wasmBase64}";
        const wasmBytes = Uint8Array.from(atob(wasmBase64), c => c.charCodeAt(0));
        const jsContent = ${JSON.stringify(jsText)}; 
        const jsBlob = new Blob([jsContent], {type: "application/javascript"});
        const jsUrl = URL.createObjectURL(jsBlob);
        const { default: init, ...bindings } = await import(jsUrl);
        const wasm = await init(wasmBytes);
        window.wasmBindings = bindings;
        dispatchEvent(new CustomEvent("TrunkApplicationStarted", {detail: {wasm}}));
    } catch (e) {
        console.error("Bundle init failed:", e);
        document.body.innerHTML = '<div style="color:white; background:red; padding:20px;">Failed to start bundled app: ' + e + '</div>';
    }
})();
        `;

        doc.body.appendChild(loaderScript);

        // trigger download
        console.log("triggering download...");
        const finalHtml = "<!doctype html>\n" + doc.documentElement.outerHTML;
        const downloadBlob = new Blob([finalHtml], {type: "text/html"});
        const downloadUrl = URL.createObjectURL(downloadBlob);
        
        const a = document.createElement("a");
        a.href = downloadUrl;
        a.download = "zcb3_bundled.html";
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(downloadUrl);
    } catch (e) {
        console.error("Bundling failed:", e);
        alert("Bundling failed: " + e.message);
    }
}
