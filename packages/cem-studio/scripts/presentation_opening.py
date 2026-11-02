"""Read-only native opening audit. Full exports remain in ignored reports."""
import argparse
import hashlib
import json
from pathlib import Path
import posixpath
import xml.etree.ElementTree as ET
import zipfile

NS = {'p': 'http://schemas.openxmlformats.org/presentationml/2006/main',
      'r': 'http://schemas.openxmlformats.org/officeDocument/2006/relationships'}
P = '{' + NS['p'] + '}'
DECK = '1v5JEclFycuXuYk5Mnpwb0alxdKIi30J6XgvLBmawxI4'
EXPECTED = [
    ('cem_teaser_thought', 0, 3000, 'fade', ()),
    ('cem_teaser_pause_1', 3000, 3000, 'fade', ()),
    ('cem_teaser_pause_2', 6000, 4100, 'fade', ()),
    ('cem_teaser_dna', 10100, 3000, 'resize', ('ppt_h', 'ppt_w')),
    ('cem_teaser_pause_3', 13100, 5000, 'fade', ()),
    ('cem_teaser_pause_4', 18100, 5000, 'fade', ()),
    ('cem_teaser_cem', 23100, 2000, 'fade', ()),
]


def bind_shapes(native, elements, result=None):
    result = {} if result is None else result
    assert len(native) == len(elements), 'Export/native shape count changed'
    for item, element in zip(native, elements):
        expected_tag = 'grpSp' if 'elementGroup' in item else 'cxnSp' if 'line' in item else 'sp' if 'shape' in item else 'pic'
        assert element.tag == P + expected_tag, 'Export/native shape type changed'
        owner = element.find('./*/p:cNvPr', NS)
        assert owner is not None and owner.get('id') not in result, 'Missing/duplicate export shape owner'
        result[owner.get('id')] = item['objectId']
        if 'elementGroup' in item:
            bind_shapes(item['elementGroup']['children'], list(element)[2:], result)
    return result


def inspect_timing(slide, owners):
    timing = slide.find('p:timing', NS)
    assert timing is not None, 'No opening animation'
    main = timing.find('.//p:cTn[@nodeType="mainSeq"]', NS)
    assert main is not None and main.get('id'), 'Missing main sequence'
    effects = []

    def walk(node, start=0, trigger=None, automatic=False):
        if node.tag == P + 'cTn':
            conditions = node.findall('p:stCondLst/p:cond', NS)
            numbers = [int(cond.get('delay')) for cond in conditions if cond.get('delay', '').isdigit()]
            start += min(numbers, default=0)
            automatic = automatic or any(cond.get('evt') == 'onBegin' and cond.get('delay') == '0'
                                         and cond.find('p:tn', NS) is not None
                                         and cond.find('p:tn', NS).get('val') == main.get('id') for cond in conditions)
            if node.get('nodeType') in ('afterEffect', 'withEffect', 'clickEffect'):
                trigger = node.get('nodeType')
        if node.tag in (P + 'animEffect', P + 'anim'):
            target = node.find('p:cBhvr/p:tgtEl/p:spTgt', NS)
            duration = node.find('p:cBhvr/p:cTn', NS)
            assert target is not None and target.get('spid') in owners, 'Unowned animation target'
            assert duration is not None and duration.get('dur', '').isdigit(), 'Unbounded animation duration'
            kind = node.get('filter') if node.tag == P + 'animEffect' else 'resize'
            assert kind in ('fade', 'resize'), 'Unsupported opening effect'
            if kind == 'fade':
                assert node.get('transition') == 'in', 'Unexpected exit effect'
            properties = tuple(element.text for element in node.findall('p:cBhvr/p:attrNameLst/p:attrName', NS))
            if kind == 'resize':
                assert len(properties) == 1 and properties[0] in ('ppt_w', 'ppt_h'), 'Unexpected resize property'
                values = node.findall('p:tavLst/p:tav', NS)
                assert [entry.get('tm') for entry in values] == ['0', '100000'], 'Unexpected zoom keyframes'
                assert [entry.find('p:val/p:strVal', NS).get('val') for entry in values] == ['0', '#' + properties[0]], 'Unexpected zoom values'
            effects.append({'target': owners[target.get('spid')], 'startMs': start,
                            'durationMs': int(duration.get('dur')), 'effect': kind,
                            'properties': properties, 'trigger': trigger, 'automatic': automatic})
        elif node.tag.startswith(P + 'anim'):
            raise AssertionError('Unsupported additional animation behavior')
        for child in node:
            walk(child, start, trigger, automatic)

    walk(timing)
    combined = []
    for effect in effects:
        if combined and all(combined[-1][key] == effect[key] for key in ('target', 'startMs', 'durationMs', 'effect')):
            combined[-1]['properties'] += effect['properties']
        else:
            combined.append(effect.copy())
    actual = [(e['target'], e['startMs'], e['durationMs'], e['effect'], tuple(sorted(e['properties']))) for e in combined]
    assert actual == EXPECTED, 'Opening timing/effect ownership changed'
    assert all(e['trigger'] == 'afterEffect' and e['automatic'] for e in combined), 'Opening requires a click or lost onBegin ownership'
    return combined


def verify_opening(raw, pptx):
    deck = raw.get('structuredContent', raw)
    assert deck['presentationId'] == DECK and len(deck['slides']) == 30, 'Wrong deck'
    assert deck['slides'][0]['objectId'] == 'p1', 'Opening order changed'
    with zipfile.ZipFile(pptx) as package:
        presentation = ET.fromstring(package.read('ppt/presentation.xml'))
        slide_ids = presentation.findall('p:sldIdLst/p:sldId', NS)
        assert len(slide_ids) == 30, 'Export slide count changed'
        relation = slide_ids[0].get('{' + NS['r'] + '}id')
        relationships = ET.fromstring(package.read('ppt/_rels/presentation.xml.rels'))
        target = next(item.get('Target') for item in relationships if item.get('Id') == relation)
        path = posixpath.normpath(posixpath.join('ppt', target))
        assert path.startswith('ppt/slides/'), 'Wrong first slide relationship'
        slide_bytes = package.read(path)
    slide = ET.fromstring(slide_bytes)
    owners = bind_shapes(deck['slides'][0]['pageElements'], list(slide.find('p:cSld/p:spTree', NS))[2:])
    groups = {e['objectId']: len(e['elementGroup']['children']) for e in deck['slides'][0]['pageElements'] if 'elementGroup' in e}
    assert groups == {'cem_teaser_dna': 53, 'cem_teaser_cem': 11, 'cem_teaser_thought': 3}, 'Current opening artwork changed'
    return {'schemaVersion': 1, 'presentationId': DECK, 'nativeRevision': deck['revisionId'],
            'slideId': 'p1', 'slideCount': 30, 'groupObjectCounts': groups,
            'effects': inspect_timing(slide, owners), 'completeMs': 25100,
            'exportSha256': hashlib.sha256(Path(pptx).read_bytes()).hexdigest(),
            'slideXmlSha256': hashlib.sha256(slide_bytes).hexdigest(),
            'mainDeckMutated': False, 'credentialStorageAccessed': False}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('native_read'); parser.add_argument('pptx'); parser.add_argument('report')
    args = parser.parse_args()
    result = verify_opening(json.loads(Path(args.native_read).read_text()), args.pptx)
    Path(args.report).write_text(json.dumps(result, indent=2) + '\n')
    print('Verified native opening: automatic start, Thought/DNA/CEM and 25100 ms completion.')
