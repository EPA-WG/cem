import { componentRuntimeErrors } from '@epa-wg/cem-site/components-runtime';

const errors = [];
errors.push(...componentRuntimeErrors);

try {
    const root = document.querySelector('[data-search-index]');
    const field = document.querySelector('[data-search-field]');
    const action = document.querySelector('[data-search-action]');
    const input = await waitForElement('[data-search-field] input');
    const actionButton = await waitForElement('[data-search-action] button');
    const status = document.querySelector('[data-search-status]');
    const results = document.querySelector('[data-search-results]');
    const documents = [...results.querySelectorAll(':scope > [data-search-document]')].map((item) => ({
        route: item.dataset.searchDocument,
        title: item.querySelector(':scope > a').textContent.trim(),
        summary: item.querySelector(':scope > p').textContent.trim(),
        headings: [...item.querySelectorAll(':scope > ul a')].map((link) => ({
            id: new URL(link.href).hash.slice(1),
            text: link.textContent.trim(),
        })),
    }));
    if (!Array.isArray(documents)) {
        throw new Error('search index must be an array');
    }

    const search = (rawQuery, updateLocation = false) => {
        const query = rawQuery.trim();
        const terms = query.toLocaleLowerCase().split(/\s+/u).filter(Boolean);
        const matches = documents.filter((document) => {
            const text = [document.title, document.summary, ...document.headings.map(({ text }) => text)]
                .join(' ')
                .toLocaleLowerCase();
            return terms.every((term) => text.includes(term));
        });
        renderResults(results, matches, terms);
        status.textContent =
            terms.length === 0
                ? `${matches.length} searchable documents`
                : `${matches.length} result${matches.length === 1 ? '' : 's'} for “${query}”`;
        if (updateLocation) {
            const url = new URL(globalThis.location.href);
            if (query) {
                url.searchParams.set('q', query);
            } else {
                url.searchParams.delete('q');
            }
            globalThis.history.replaceState(null, '', url);
        }
        globalThis.__cemSiteSearch = {
            done: true,
            errors,
            documentCount: documents.length,
            query,
            resultCount: matches.length,
        };
    };

    const initialQuery = new URL(globalThis.location.href).searchParams.get('q') ?? '';
    input.value = initialQuery;
    input.addEventListener('input', () => search(input.value, true));
    actionButton.addEventListener('click', () => search(input.value, true));
    field.closest('form').addEventListener('submit', (event) => {
        event.preventDefault();
        search(input.value, true);
    });
    action.setAttribute('aria-controls', 'search-results');
    search(initialQuery);
} catch (error) {
    errors.push(error instanceof Error ? error.message : String(error));
    globalThis.__cemSiteSearch = { done: true, errors, documentCount: 0, query: '', resultCount: 0 };
}

function renderResults(root, documents, terms) {
    root.replaceChildren(
        ...documents.map((record) => {
            const matchingHeadings =
                terms.length === 0
                    ? record.headings
                    : record.headings
                          .filter(({ text }) => {
                              const normalized = text.toLocaleLowerCase();
                              return terms.every((term) => normalized.includes(term));
                          })
                          .sort((left, right) => {
                              const phrase = terms.join(' ');
                              return (
                                  Number(right.text.toLocaleLowerCase() === phrase) -
                                  Number(left.text.toLocaleLowerCase() === phrase)
                              );
                          });
            const primaryHeading = matchingHeadings[0] ?? record.headings[0];
            const item = document.createElement('li');
            item.dataset.searchDocument = record.route;
            const link = document.createElement('a');
            link.href = `${record.route}#${primaryHeading.id}`;
            link.textContent = record.title;
            const summary = document.createElement('p');
            summary.textContent = record.summary;
            item.append(link, summary);
            if (matchingHeadings.length > 0) {
                const headings = document.createElement('ul');
                headings.append(
                    ...matchingHeadings.map((heading) => {
                        const headingItem = document.createElement('li');
                        const headingLink = document.createElement('a');
                        headingLink.href = `${record.route}#${heading.id}`;
                        headingLink.textContent = heading.text;
                        headingItem.append(headingLink);
                        return headingItem;
                    }),
                );
                item.append(headings);
            }
            return item;
        }),
    );
}

async function waitForElement(selector, attempts = 120) {
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        const element = document.querySelector(selector);
        if (element) {
            return element;
        }
        await new Promise((resolve) => requestAnimationFrame(resolve));
    }
    throw new Error(`timed out waiting for ${selector}`);
}
