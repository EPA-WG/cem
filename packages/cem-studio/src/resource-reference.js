/** Serialized address for a native project resource identity. Names and export paths are mutable metadata.
 * Legacy manifests bind their original address once at import; new resources use their stable ID.
 * @param {{rootUri: string}} project
 * @param {{id: string, path: string, referencePath?: string}} resource
 */
export function projectResourceUri(project, resource) {
    return project.rootUri + (resource.referencePath ?? resource.path);
}
