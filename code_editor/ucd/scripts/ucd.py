#!/usr/bin/python3

"""Parsing UCD data files."""

import re
import sys


class Error(Exception):
    """Error base class."""


def parse_code_point(string):
    """Parses a Unicode code point.

    Code points are expressed as hexadecimal numbers with four to six digits.
    """
    if len(string) < 4 or len(string) > 6:
        raise Error("invalid code point %s" % string)
    try:
        value = int(string, 16)
    except ValueError:
        raise Error("invalid code point %s" % string)
    if value < 0 or value > sys.maxunicode:
        raise Error("invalid code point %s" % string)
    return value


def parse_code_point_range(string):
    """Parses a range of Unicode code points.

    A range of code points is specified either by the form "X..Y", where X is
    less than or equal to Y, or by the form "X", which is short for "X..X".
    """
    bounds = string.split("..")
    if len(bounds) == 1:
        code_point = parse_code_point(bounds[0])
        return range(code_point, code_point + 1)
    elif len(bounds) == 2:
        first = parse_code_point(bounds[0])
        last = parse_code_point(bounds[1])
        if first > last:
            raise Error("invalid code point range %s" % string)
        return range(first, last + 1)
    else:
        raise Error("invalid code point range %s" % string)


def parse_character_name(string):
    """Parses a Unicode character name.

    For backward compatibility, ranges in the file UnicodeData.txt are
    specified by entries for the start and end characters of the range, rather
    than by the form "X..Y". The start character is indicated by a range
    identifier, followed by a comma and the string "First", in angle brackets.
    This line takes the place of a regular character name in field 1 for that
    line. The end character is indicated on the next line with the same range
    identifier, followed by a comma and the string "Last", in angle brackets.
    """
    match = re.match("<(.*), (First|Last)>", string)
    if match is not None:
        return match.groups()
    return (string, None)


def read_unicode_data(filename, expected_field_count):
    """A reader for Unicode data files.

    The reader strips out comments and whitespace, and skips empty lines. For
    non-empty lines, the reader splits the line into fields, checks if the
    line has the expected number of fields, and strips out whitespace for each
    field.

    The reader also takes care of parsing code point ranges. Unicode data
    files specify these in two different ways, either by the form "X..Y", or
    by entries for the start and end characters of the range.
    """
    file = open(filename, encoding="utf8")
    first = None
    expected_name = None
    for lineno, line in enumerate(file):
        # Strip out comments and whitespace, and skip empty lines.
        hash = line.find("#")
        if hash >= 0:
            line = line[:hash]
        line = line.strip()
        if not line:
            continue

        try:
            # Split the line into fields, check if the line has the expected
            # number of fields, and strip out whitespace for each field.
            fields = [field.strip() for field in line.split(";")]
            if len(fields) != expected_field_count:
                raise ValueError("invalid number of fields %d" % len(fields))

            # Unicode data files specify code point ranges in two different
            # ways, either by the form "X..Y", or by entries for the start and
            # end characters of the range.
            code_points = parse_code_point_range(fields[0])
            (name, first_or_last) = parse_character_name(fields[1])
            if expected_name is None:
                if first_or_last == "First":
                    # The current line is the first entry for a range.
                    # Remember it and continue with the next line.
                    if len(code_points) != 1:
                        raise ValueError("invalid First line")
                    expected_name = name
                    first = code_points[0]
                    continue
            else:
                # If the previous line was the first entry for a range, the
                # current line should be the last entry for that range.
                if name != expected_name or first_or_last != "Last" or len(
                        code_points) != 1 or code_points[0] < first:
                    raise ValueError("invalid Last line")
                code_points = range(first, code_points[0] + 1)
                fields[1] = name
                first = None
                expected_name = None
        except Exception as e:
            e.args = ("%s:%d: " % (filename, lineno + 1), *e.args)
            raise e.with_traceback(e.__traceback__)
        fields[0] = code_points
        yield fields
        lineno += 1


def extended_pictographic(ucd_dir):
    """Returns a list of code point ranges for which the Extended_Pictographic property is True."""
    extended_pictographic = []
    for [code_points, value] in read_unicode_data(ucd_dir + "/emoji/emoji-data.txt", 2):
        if value != "Extended_Pictographic":
            continue
        extended_pictographic.extend(code_points)
    return extended_pictographic


def grapheme_cluster_break(ucd_dir):
    """Returns a dict mapping Grapheme_Cluster_Break categories to lists of code point ranges."""
    grapheme_cluster_break = {}
    for [code_points, value] in read_unicode_data(ucd_dir + "/auxiliary/GraphemeBreakProperty.txt", 2):
        grapheme_cluster_break.setdefault(value, []).extend(code_points)
    return grapheme_cluster_break


def word_break(ucd_dir):
    """Returns a dict mapping Word_Break categories to lists of code point ranges."""
    word_break = {}
    for [code_points, value] in read_unicode_data(ucd_dir + "/auxiliary/WordBreakProperty.txt", 2):
        word_break.setdefault(value, []).extend(code_points)
    return word_break