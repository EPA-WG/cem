import unittest
import xml.etree.ElementTree as ET
from presentation_opening import EXPECTED, P, bind_shapes, inspect_timing


def fixture():
    slide = ET.Element(P + 'sld'); timing = ET.SubElement(slide, P + 'timing')
    main = ET.SubElement(timing, P + 'cTn', nodeType='mainSeq', id='2')
    start = ET.SubElement(main, P + 'stCondLst'); cond = ET.SubElement(start, P + 'cond', evt='onBegin', delay='0')
    ET.SubElement(cond, P + 'tn', val='2')
    owners = {}
    for index, (target, delay, duration, kind, properties) in enumerate(EXPECTED):
        owners[str(index)] = target
        effect = ET.SubElement(main, P + 'cTn', nodeType='afterEffect')
        ET.SubElement(ET.SubElement(effect, P + 'stCondLst'), P + 'cond', delay=str(delay))
        for name in properties or (None,):
            animation = ET.SubElement(effect, P + ('anim' if name else 'animEffect'), filter='fade', transition='in')
            behavior = ET.SubElement(animation, P + 'cBhvr')
            ET.SubElement(behavior, P + 'cTn', dur=str(duration))
            ET.SubElement(ET.SubElement(behavior, P + 'tgtEl'), P + 'spTgt', spid=str(index))
            if name:
                ET.SubElement(ET.SubElement(behavior, P + 'attrNameLst'), P + 'attrName').text = name
                values = ET.SubElement(animation, P + 'tavLst')
                for time, value in [('0', '0'), ('100000', '#' + name)]:
                    item = ET.SubElement(values, P + 'tav', tm=time)
                    ET.SubElement(ET.SubElement(item, P + 'val'), P + 'strVal', val=value)
    return slide, owners


class OpeningTests(unittest.TestCase):
    def test_current_native_timeline(self):
        slide, owners = fixture(); effects = inspect_timing(slide, owners)
        self.assertEqual(len(effects), 7); self.assertTrue(all(e['automatic'] for e in effects))

    def test_timing_drift(self):
        slide, owners = fixture(); slide.find('.//' + P + 'animEffect/' + P + 'cBhvr/' + P + 'cTn').set('dur', '500')
        with self.assertRaisesRegex(AssertionError, 'timing/effect'): inspect_timing(slide, owners)

    def test_click_required(self):
        slide, owners = fixture(); slide.findall('.//' + P + 'cTn')[1].set('nodeType', 'clickEffect')
        with self.assertRaisesRegex(AssertionError, 'requires a click'): inspect_timing(slide, owners)

    def test_wrong_on_begin_owner(self):
        slide, owners = fixture(); slide.find('.//' + P + 'tn').set('val', 'foreign')
        with self.assertRaisesRegex(AssertionError, 'onBegin ownership'): inspect_timing(slide, owners)

    def test_unowned_target(self):
        slide, owners = fixture(); owners.pop('0')
        with self.assertRaisesRegex(AssertionError, 'Unowned'): inspect_timing(slide, owners)

    def test_dna_missing_height(self):
        slide, owners = fixture(); dna = slide.findall('.//' + P + 'cTn[@nodeType="afterEffect"]')[3]; dna.remove(list(dna)[-1])
        with self.assertRaisesRegex(AssertionError, 'timing/effect'): inspect_timing(slide, owners)

    def test_shape_count_mismatch(self):
        with self.assertRaisesRegex(AssertionError, 'shape count'): bind_shapes([{'objectId': 'id'}], [])

    def test_shape_type_mismatch(self):
        with self.assertRaisesRegex(AssertionError, 'shape type'): bind_shapes([{'objectId': 'id', 'shape': {}}], [ET.Element(P + 'pic')])

    def test_changed_zoom_values(self):
        slide, owners = fixture(); slide.find('.//' + P + 'tav/' + P + 'val/' + P + 'strVal').set('val', '99')
        with self.assertRaisesRegex(AssertionError, 'zoom values'): inspect_timing(slide, owners)


if __name__ == '__main__': unittest.main()
