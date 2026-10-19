#!/usr/bin/env python3
"""Guard the reference harness's narrowly pinned expected rejections."""
import unittest
from google_differential import EXPECTED_REJECTIONS, PHOTO_TIME_REJECTION, expected_build_rejection


class RejectionContract(unittest.TestCase):
    def test_exact_fixed_input_and_error_are_required(self):
        for digest, name in EXPECTED_REJECTIONS.items():
            self.assertTrue(expected_build_rejection(name, digest, PHOTO_TIME_REJECTION, None))
            for changed_name, changed_digest, changed_error, rebuilt in [
                ('another-honor.jpg', digest, PHOTO_TIME_REJECTION, None),
                (name, '0'*64, PHOTO_TIME_REJECTION, None),
                (name, digest, PHOTO_TIME_REJECTION+'; another issue', None),
                (name, digest, 'other error', None),
                (name, digest, None, 'unexpectedly-rebuilt.jpg'),
                (name, digest, PHOTO_TIME_REJECTION, 'unexpected-output.jpg'),
            ]:
                with self.assertRaises(AssertionError):
                    expected_build_rejection(changed_name, changed_digest, changed_error, rebuilt)

    def test_other_inputs_must_build(self):
        self.assertFalse(expected_build_rejection('honor-positive.jpg', 'f'*64, None, 'built.jpg'))
        with self.assertRaises(AssertionError):
            expected_build_rejection('honor-positive.jpg', 'f'*64, PHOTO_TIME_REJECTION, None)


if __name__ == '__main__':
    unittest.main()
