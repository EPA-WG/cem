import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { whenCemSourceRendered } from '../../.storybook/preview.js';

export default { title: 'CEM Elements/Root Gallery', tags: ['test'] } satisfies Meta;
const SOURCE_TAG = 'story-root-gallery';

export const EveryAuthoredSample: StoryObj = {
    render: () => {
        const root = document.createElement('section');
        const declaration = document.createElement('cem-element');
        declaration.hidden = true;
        declaration.setAttribute('tag', SOURCE_TAG);
        declaration.setAttribute('src', new URL('../../index.html', import.meta.url).href);
        root.append(declaration, document.createElement(SOURCE_TAG));
        return root;
    },
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector<HTMLElement>(SOURCE_TAG);
        if (!host) throw new Error('Missing root-gallery host');
        await whenCemSourceRendered(host);
        expect(host.querySelector('dce-link a')).toHaveTextContent('link 😃');
        expect(host.querySelector('dce-link i')).toBeNull();
        expect(host.querySelector('dce-1-slot')).toHaveTextContent('🐇❤️🥕');
        const repeated = host.querySelectorAll('dce-2-slots');
        expect(repeated).toHaveLength(2);
        expect(repeated[0].querySelectorAll('i[slot="slot2"]')).toHaveLength(2);
        expect(repeated[0].querySelector('input')).toHaveAttribute('placeholder', '🐇❤️🥕');
        expect(repeated[1].querySelector('input')).toHaveAttribute('placeholder', '🐇❤️🐇');
        expect(Array.from(host.querySelectorAll('dce-3-slot'), element => element.textContent?.replace(/\s+/gu, ' ').trim()))
            .toEqual(['1 😃 2 😃', '1 🥕 2 🥕', '1 ✌️ 2 ✌️']);
        expect(host.querySelector('dce-4-slot')).toHaveTextContent('1 🥕 2 🥕');
        expect(Array.from(host.querySelectorAll('greet-element'), element => element.textContent?.trim()))
            .toEqual(['Hello World!', '👋 World!']);
        const bulbasaur = host.querySelector('pokemon-tile[title="bulbasaur"]');
        if (!bulbasaur) throw new Error('Missing authored bulbasaur');
        expect(bulbasaur.querySelector('h3')).toHaveTextContent('bulbasaur');
        expect(bulbasaur).toHaveTextContent('Smile as: 👼');
        expect(bulbasaur.querySelector('p')).toHaveTextContent('Bulbasaur is a cute Pokémon');
        expect(Array.from(bulbasaur.querySelectorAll('button'), element => element.textContent?.trim()))
            .toEqual(['ivysaur', 'venusaur']);
        expect(Array.from(bulbasaur.querySelectorAll('button img'), element => element.getAttribute('alt')))
            .toEqual(['ivysaur', 'venusaur']);
        expect(bulbasaur.querySelector('img')).toHaveAttribute('src',
            'https://unpkg.com/pokeapi-sprites@2.0.2/sprites/pokemon/other/dream-world/1.svg');
        const ninetales = host.querySelector('pokemon-tile[title="ninetales"]');
        if (!ninetales) throw new Error('Missing authored ninetales');
        expect(ninetales.querySelector('h3')).toHaveTextContent('ninetales');
        expect(ninetales).not.toHaveTextContent('Smile as:');
        expect(ninetales.querySelector('p')).toHaveTextContent('description is not available');
        expect(ninetales.querySelector('button')).toHaveTextContent('vulpix');
        expect(Array.from(ninetales.querySelectorAll('button'), element => element.textContent?.trim()))
            .toEqual(['vulpix']);
        expect(ninetales.querySelector('button img')).toHaveAttribute('alt', 'vulpix');
    },
};
