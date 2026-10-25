import { verifyCemqlProject } from './verify-cemql-project.mjs';
import { verifyXpathProject } from './verify-xpath-project.mjs';

/** Ordinary native data-query projects reuse Studio's shared workspace controls. */
export async function verifyDataQueryProjects(context) {
    return { 'cemql-orders': await verifyCemqlProject(context), 'xpath-catalog': await verifyXpathProject(context) };
}
