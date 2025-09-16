import { existsSync } from "fs";
import { dirname, extname, basename, join } from "path";
import { fileURLToPath } from "url";

const extensions = ["mjs", "js", "json"];

function findExtension(specifier, parentURL) {
    if (extname(specifier)) return "";
    for (const ext of extensions) {
        const candidate = join(dirname(fileURLToPath(parentURL)), specifier + "." + ext);
        if (existsSync(candidate)) return "." + ext;
    }
    return "";
}

export function resolve(specifier, context, nextResolve) {
    if (
        (specifier.startsWith("./") || specifier.startsWith("../") || specifier.startsWith("/")) &&
        !extname(basename(specifier))
    ) {
        const ext = findExtension(specifier, context.parentURL);
        if (ext) {
            return nextResolve(specifier + ext, context);
        }
    }
    return nextResolve(specifier, context);
}
