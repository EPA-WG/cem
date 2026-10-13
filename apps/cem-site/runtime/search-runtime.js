import { componentRuntimeErrors, mountSiteView, viewErrors } from '@epa-wg/cem-site/components-runtime';

const errors = [...componentRuntimeErrors];
try {
    const root = document.querySelector('[data-search-index]');
    // Native CEM-ML publishes the complete index as useful static HTML. Read
    // its scalar control data once before mounting the interactive declaration.
    const documents = [...root.querySelectorAll('[data-search-results] > [data-search-document]')].map(item => ({
        route: item.getAttribute('data-search-document'),
        title: item.querySelector(':scope > a').textContent.trim(),
        summary: item.querySelector(':scope > p').textContent.trim(),
        headings: [...item.querySelectorAll(':scope > ul a')].map(link => ({
            id: new URL(link.href).hash.slice(1), text: link.textContent.trim(),
        })),
    }));
    let state = search(new URL(globalThis.location.href).searchParams.get('q') ?? '');
    let revision = 0;
    let view = null;
    const publish = async (rawQuery) => {
        const current = ++revision;
        globalThis.__cemSiteSearch = { ...globalThis.__cemSiteSearch, done: false };
        state = search(rawQuery);
        const url = new URL(globalThis.location.href);
        if (state.query) url.searchParams.set('q', state.query);
        else url.searchParams.delete('q');
        globalThis.history.replaceState(globalThis.history.state, '', url);
        if (!view) return;
        view.update({ state });
        await view.whenSettled();
        if (current === revision) report();
    };
    view = await mountSiteView(root, 'cem-site-search', new URL('./search-view.xhtml#cem-site-search', import.meta.url).href,
        state, {
            search: ({ value }) => publish(value ?? ''),
            'submit-search': ({ fields }) => publish(fields?.q?.[0] ?? state.rawQuery),
        });
    view.update({ state });
    await view.whenSettled();
    report();

    function search(rawQuery) {
        const query = rawQuery.trim();
        const terms = query.toLocaleLowerCase().split(/\s+/u).filter(Boolean);
        const matches = documents.filter(record => {
            const text = [record.title, record.summary, ...record.headings.map(({ text }) => text)].join(' ').toLocaleLowerCase();
            return terms.every(term => text.includes(term));
        }).map(record => {
            const headings = terms.length === 0 ? record.headings : record.headings
                .filter(({ text }) => terms.every(term => text.toLocaleLowerCase().includes(term)))
                .sort((left, right) => Number(right.text.toLocaleLowerCase() === terms.join(' '))
                    - Number(left.text.toLocaleLowerCase() === terms.join(' ')));
            return { ...record, headings, primaryHeading: headings[0] ?? record.headings[0] };
        });
        return { rawQuery, query, matches };
    }
    function report() {
        globalThis.__cemSiteSearch = { done: true, errors: [...errors, ...viewErrors(view)],
            documentCount: documents.length, query: state.query, resultCount: state.matches.length };
    }
} catch (error) {
    errors.push(error instanceof Error ? error.message : String(error));
    globalThis.__cemSiteSearch = { done: true, errors, documentCount: 0, query: '', resultCount: 0 };
}
