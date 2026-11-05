export function import_recording() {
    return new Promise((resolve, reject) => {
        const picker = document.createElement("input");
        picker.type = "file";
        picker.accept = ".treplay";
        picker.oncancel = () => resolve(new Uint8Array());
        picker.onchange = async () => {
            const file = picker.files[0];
            if (!file) { resolve(new Uint8Array()); return; }
            if (file.size > 128 * 1024 * 1024) {
                reject(new Error("Recording exceeds the 128 MiB limit"));
                return;
            }
            try { resolve(new Uint8Array(await file.arrayBuffer())); }
            catch (error) { reject(error); }
        };
        picker.click();
    });
}

export function export_recording(name, bytes) {
    const url = URL.createObjectURL(new Blob([bytes], { type: "application/octet-stream" }));
    const link = document.createElement("a");
    link.href = url;
    link.download = name;
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
}
