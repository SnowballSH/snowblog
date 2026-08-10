import { site } from '$lib/site.js';
import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = () => ({ site: site() });
