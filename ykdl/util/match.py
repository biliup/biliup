#!/usr/bin/env python
# -*- coding: utf-8 -*-

import re

def match1(text, *patterns):
    """Scans through a string for substrings matched some patterns (first-subgroups only).

    Args:
        text: A string to be scanned.
        patterns: Arbitrary number of regex patterns.

    Returns:
        When matches, returns first-subgroups from first match.
        When no matches, return None
    """

    for pattern in patterns:
        try:
            match = re.search(pattern, text)
        except(TypeError):
            match = re.search(pattern, str(text))
        if match:
            return match.group(1)
    return None


def matchall(text, patterns):
    """Scans through a string for substrings matched some patterns.

    Args:
        text: A string to be scanned.
        patterns: a list of regex pattern.

    Returns:
        a list if matched. empty if not.
    """

    ret = []
    for pattern in patterns:
        try:
            match = re.findall(pattern, text)
        except(TypeError):
            match = re.findall(pattern, str(text))
        ret += match

    return ret
