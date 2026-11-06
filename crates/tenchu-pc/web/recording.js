export function import_recording(extension, limit) {
    return new Promise((resolve, reject) => {
        const picker = document.createElement("input");
        picker.type = "file";
        picker.accept = extension;
        picker.oncancel = () => resolve(new Uint8Array());
        picker.onchange = async () => {
            const file = picker.files[0];
            if (!file) { resolve(new Uint8Array()); return; }
            if (file.size > limit) {
                reject(new Error(`File exceeds the ${limit / 1048576} MiB limit`));
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
