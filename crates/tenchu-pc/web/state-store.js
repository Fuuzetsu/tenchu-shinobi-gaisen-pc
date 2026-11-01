function open() {
    return new Promise((resolve, reject) => {
        const request = indexedDB.open("tenchu-savestates", 1);
        request.onupgradeneeded = () => request.result.createObjectStore("slots");
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
        request.onblocked = () => reject(new Error("Close other game tabs before opening savestates"));
    });
}

export async function state_write(slot, bytes) {
    const db = await open();
    try {
        await new Promise((resolve, reject) => {
            const transaction = db.transaction("slots", "readwrite");
            transaction.objectStore("slots").put(bytes, slot);
            transaction.oncomplete = () => resolve();
            transaction.onabort = () => reject(transaction.error || new Error("Savestate write cancelled"));
            transaction.onerror = () => reject(transaction.error);
        });
    } finally {
        db.close();
    }
}

export async function state_read(slot) {
    const db = await open();
    try {
        return await new Promise((resolve, reject) => {
            const request = db.transaction("slots", "readonly").objectStore("slots").get(slot);
            request.onsuccess = () => {
                const bytes = request.result;
                if (!(bytes instanceof Uint8Array)) {
                    reject(new Error(`Savestate slot ${slot + 1} is empty`));
                } else if (bytes.length > 128 * 1024 * 1024) {
                    reject(new Error("Savestate exceeds the 128 MiB limit"));
                } else {
                    resolve(bytes);
                }
            };
            request.onerror = () => reject(request.error);
        });
    } finally {
        db.close();
    }
}
