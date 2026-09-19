<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
    xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:g="cem:generic-data"
    xmlns:j="http://www.w3.org/2005/xpath-functions" xmlns:import="urn:cem:import"
    xmlns:source="urn:cem:source" xmlns:err="http://www.w3.org/2005/xqt-errors"
    version="3.0">
    <!-- Named entry parameters are scalar host controls. Only import reads source syntax. -->
    <xsl:template name="viewer" match="/">
        <xsl:param name="initial" select="string(.)"/>
        <xsl:param name="source" select="$initial"/>
        <xsl:param name="format" select="'xml'"/>
        <xsl:param name="column" select="''"/>
        <xsl:param name="direction" select="'ascending'"/>
        <xsl:param name="mode" select="'text'"/>
        <xsl:param name="selected" select="''"/>
        <xsl:param name="presentation" select="map {}"/>
        <xsl:variable name="ui" select="map {'column':$column, 'direction':$direction, 'mode':$mode, 'selected':$selected, 'presentation':$presentation}"/>
        <style>    article { min-width: 0; }
    label { display: block; margin-block: .5rem; }
    textarea { box-sizing: border-box; width: 100%; min-height: 9rem;
        font-family: monospace; resize: vertical; }
    select, input { max-width: 100%; }
    .toolbar { display: flex; flex-wrap: wrap; gap: .5rem; align-items: center; }
    .table-scroll { overflow: auto; max-height: 22rem; }
    table { width: 100%; border-collapse: collapse; }
    th, td { padding: .35rem; border: 1px solid var(--cem-table-border, #999);
        text-align: start; overflow-wrap: anywhere; }
    .value { white-space: pre-wrap; }
    tr[aria-selected=true] { background: var(--cem-table-selected, #d3f5dd); color: #18251c; }
    summary { cursor: pointer; overflow-wrap: anywhere; }
    details { margin-block: .5rem; }
    ul { padding-inline-start: 1.25rem; }
    [role=alert] { color: var(--cem-table-error, #a51d32); overflow-wrap: anywhere; }
</style>
        <article class="demo-card">
            <label><xsl:text>Source (</xsl:text><xsl:value-of select="map {'xml':'XML', 'json':'JSON', 'csv':'CSV', 'yaml':'YAML'}?($format)"/><xsl:text>)</xsl:text>
                <textarea aria-label="Source" spellcheck="false" slice="source" slice-event="change" slice-value="$target.value"><xsl:value-of select="$source"/></textarea>
            </label>
            <p>Edit, then Tab out to inspect.</p>
            <xsl:try>
                <xsl:variable name="document" select="
                    if ($format = 'xml') then parse-xml($source)
                    else if ($format = 'json') then json-to-xml($source)
                    else if ($format = 'csv') then import:parse-csv($source, map {'header':'present'})
                    else import:parse-yaml($source)"/>
                <xsl:variable name="columns" select="distinct-values(
                    for $entry in $document/descendant::* return
                        if (namespace-uri($entry) = 'cem:generic-data') then
                            if (local-name($entry) = 'property') then string($entry/@name)
                            else if (local-name($entry) = 'array') then '#value' else ()
                        else if (namespace-uri($entry) = 'http://www.w3.org/2005/xpath-functions') then
                            ($entry/@key ! string(.), if (local-name($entry) = 'array') then '#value' else ())
                        else (
                            for $attr in $entry/@* return '@' || (if (namespace-uri($attr) = '') then '' else namespace-uri($attr) || '|') || local-name($attr),
                            '#text', (if (namespace-uri($entry) = '') then '' else namespace-uri($entry) || '|') || local-name($entry)))"/>
                <xsl:call-template name="toolbar">
                    <xsl:with-param name="initial" select="$initial"/>
                    <xsl:with-param name="ui" select="$ui"/>
                    <xsl:with-param name="columns" select="$columns"/>
                </xsl:call-template>
                <xsl:apply-templates select="$document/*" mode="inspect">
                    <xsl:with-param name="key" select="'document'"/>
                    <xsl:with-param name="ui" select="$ui"/>
                </xsl:apply-templates>
                <xsl:catch errors="err:FODC0006 err:FOJS0001 err:FOJS0003 import:invalid-source">
                    <xsl:call-template name="toolbar">
                        <xsl:with-param name="initial" select="$initial"/>
                        <xsl:with-param name="ui" select="$ui"/>
                        <xsl:with-param name="columns" select="()"/>
                    </xsl:call-template>
                    <p role="alert"><xsl:text>⚠ </xsl:text><xsl:value-of select="$err:description"/></p>
                </xsl:catch>
            </xsl:try>
        </article>
    </xsl:template>

    <xsl:template name="toolbar">
        <xsl:param name="initial"/><xsl:param name="ui"/><xsl:param name="columns"/>
        <div class="toolbar">
            <button type="button" aria-label="Reset source" title="Reset source" slice="source" slice-event="click" value="{$initial}">↺</button>
            <label>Sort column
                <select aria-label="Sort column" slice="column" slice-event="change" value="{$ui?column}">
                    <option value="">Source order</option>
                    <xsl:for-each select="$columns"><option value="{.}"><xsl:value-of select="."/></option></xsl:for-each>
                </select>
            </label>
            <label>Compare<select aria-label="Compare" slice="mode" slice-event="change" value="{$ui?mode}">
                <option value="text">A–Z</option><option value="number">1–9</option>
            </select></label>
            <label>Direction<select aria-label="Direction" slice="direction" slice-event="change" value="{$ui?direction}">
                <option value="ascending">↑</option><option value="descending">↓</option>
            </select></label>
        </div>
    </xsl:template>

    <xsl:template match="g:array | j:array" mode="inspect" priority="0">
        <xsl:param name="key"/><xsl:param name="ui"/>
        <xsl:call-template name="table">
            <xsl:with-param name="rows" select="*"/><xsl:with-param name="key" select="$key"/>
            <xsl:with-param name="ui" select="$ui"/>
        </xsl:call-template>
    </xsl:template>
    <xsl:template match="*" mode="inspect" priority="-10">
        <xsl:param name="key"/><xsl:param name="ui"/>
        <xsl:call-template name="tree">
            <xsl:with-param name="key" select="$key"/><xsl:with-param name="ui" select="$ui"/>
        </xsl:call-template>
    </xsl:template>
    <xsl:template match="text() | comment() | processing-instruction()" mode="inspect" priority="-20">
        <span class="value"><xsl:value-of select="if (self::processing-instruction()) then local-name() || (if (string(.) = '') then '' else ' ' || string(.)) else string(.)"/></span>
    </xsl:template>

    <xsl:template name="tree">
        <xsl:param name="key"/><xsl:param name="ui"/>
        <xsl:choose>
            <xsl:when test="namespace-uri() = ('cem:generic-data', 'http://www.w3.org/2005/xpath-functions') and not(local-name() = ('array', 'object', 'map'))">
                <span class="value"><xsl:choose>
                    <xsl:when test="local-name() = 'null'">null</xsl:when>
                    <xsl:when test="string(.) = ''">""</xsl:when>
                    <xsl:otherwise><xsl:value-of select="."/></xsl:otherwise>
                </xsl:choose></span>
            </xsl:when>
            <xsl:otherwise>
                <details open="open">
                    <summary><xsl:text>🌳 </xsl:text><xsl:value-of select="$key"/><xsl:text> · </xsl:text><xsl:value-of select="if (self::j:map) then 'object' else local-name()"/></summary>
                    <ul><xsl:choose>
                        <xsl:when test="namespace-uri() = ('cem:generic-data', 'http://www.w3.org/2005/xpath-functions')">
                            <xsl:for-each select="*"><li><xsl:choose>
                                <xsl:when test="self::g:property or @key">
                                    <xsl:variable name="property" select="string((@name, @key)[1])"/>
                                    <strong><xsl:value-of select="$property"/><xsl:text>: </xsl:text></strong>
                                    <xsl:apply-templates select="if (self::g:property) then * else ." mode="inspect">
                                        <xsl:with-param name="key" select="$property"/><xsl:with-param name="ui" select="$ui"/>
                                    </xsl:apply-templates>
                                </xsl:when>
                                <xsl:otherwise><xsl:apply-templates select="." mode="inspect">
                                    <xsl:with-param name="key" select="$key"/><xsl:with-param name="ui" select="$ui"/>
                                </xsl:apply-templates></xsl:otherwise>
                            </xsl:choose></li></xsl:for-each>
                        </xsl:when>
                        <xsl:otherwise>
                            <xsl:for-each select="@*"><li><xsl:text>@</xsl:text><xsl:value-of select="local-name()"/><xsl:text>: </xsl:text><xsl:value-of select="."/></li></xsl:for-each>
                            <xsl:for-each select="node()[not(self::*) and normalize-space(.) != '']">
                                <li><xsl:apply-templates select="." mode="inspect"/></li>
                            </xsl:for-each>
                            <xsl:for-each-group select="*" group-by="namespace-uri() || '|' || local-name()">
                                <li><xsl:choose>
                                    <xsl:when test="count(current-group()) gt 1">
                                        <xsl:call-template name="table">
                                            <xsl:with-param name="rows" select="current-group()"/>
                                            <xsl:with-param name="key" select="$key || '/' || local-name()"/>
                                            <xsl:with-param name="ui" select="$ui"/>
                                        </xsl:call-template>
                                    </xsl:when>
                                    <xsl:otherwise><xsl:apply-templates select="current-group()" mode="inspect">
                                        <xsl:with-param name="key" select="$key || '/' || local-name()"/>
                                        <xsl:with-param name="ui" select="$ui"/>
                                    </xsl:apply-templates></xsl:otherwise>
                                </xsl:choose></li>
                            </xsl:for-each-group>
                        </xsl:otherwise>
                    </xsl:choose></ul>
                </details>
            </xsl:otherwise>
        </xsl:choose>
    </xsl:template>

    <xsl:template name="table">
        <xsl:param name="rows"/><xsl:param name="key"/><xsl:param name="ui"/>
        <!-- These are presentation records retaining their source nodes, not document objects. -->
        <xsl:variable name="records" select="for $row in $rows return map {
            'source':$row, 'cells':
                if ($row/self::g:object) then
                    for $property in $row/* return map {'key':string($property/@name),
                        'value':if ($property/*[1]/self::g:null) then 'null' else string-join($property/*/text(), ''),
                        'subject':$property/*}
                else if ($row/self::j:map) then
                    for $property in $row/* return map {'key':string($property/@key),
                        'value':if ($property/self::j:null) then 'null' else string-join($property/text(), ''),
                        'subject':$property}
                else if (namespace-uri($row) = ('cem:generic-data', 'http://www.w3.org/2005/xpath-functions')) then
                    map {'key':'#value', 'value':string-join($row/text(), ''), 'subject':$row}
                else (
                    for $attr in $row/@* return map {
                        'key':'@' || (if (namespace-uri($attr) = '') then '' else namespace-uri($attr) || '|') || local-name($attr),
                        'value':string($attr), 'subject':()},
                    map {'key':'#text', 'value':replace(string-join($row/text(), ''), '^\s+|\s+$', ''), 'subject':()},
                    for $child in $row/* return map {
                        'key':(if (namespace-uri($child) = '') then '' else namespace-uri($child) || '|') || local-name($child),
                        'value':string-join($child/text(), ''), 'subject':$child})}"/>
        <xsl:variable name="headings" select="distinct-values($records?cells?key)"/>
        <xsl:variable name="sortable" select="for $record in $records return
            let $cells := $record?cells[?key = $ui?column]
            return let $sort := if (empty($cells)) then () else string-join($cells?value, ' ')
            return let $number := if ($sort castable as xs:double) then xs:double($sort) else ()
            return map {'source':$record?source, 'cells':$record?cells,
                'sort':$sort, 'number':if ($number = $number and abs($number) != xs:double('INF')) then $number else ()}"/>
        <details open="open">
            <summary><xsl:text>🧮 </xsl:text><xsl:value-of select="$key"/><xsl:text> · </xsl:text><xsl:value-of select="count($rows)"/><xsl:text> rows</xsl:text></summary>
            <xsl:choose>
                <xsl:when test="empty($rows)"><p>∅ Empty collection</p></xsl:when>
                <xsl:otherwise>
                    <div class="table-scroll" tabindex="0" role="region" aria-label="{$key}">
                        <table aria-label="{$key}">
                            <thead><tr><th scope="col">✓</th><xsl:for-each select="$headings"><th scope="col"><xsl:value-of select="."/></th></xsl:for-each></tr></thead>
                            <tbody><xsl:for-each select="$sortable">
                                <xsl:sort select="if ($ui?column = '') then false() else if ($ui?mode = 'number') then empty(?number) else empty(?sort)"/>
                                <xsl:sort select="if ($ui?column = '') then position() else if ($ui?mode = 'number') then (if (exists(?number)) then ?number else 0) else ?sort"
                                    order="{if ($ui?column = '') then 'ascending' else $ui?direction}"
                                    data-type="{if ($ui?column = '' or $ui?mode = 'number') then 'number' else 'text'}"/>
                                <xsl:variable name="record" select="."/>
                                <xsl:variable name="id" select="source:node-key(?source)"/>
                                <tr aria-selected="{$id = $ui?selected}">
                                    <td><button type="button" slice="selected" slice-event="click" value="{$id}"
                                        aria-label="Select source line {source:line-number(?source)}" aria-pressed="{$id = $ui?selected}">
                                        <xsl:choose><xsl:when test="$id = $ui?selected">✓</xsl:when><xsl:otherwise>○</xsl:otherwise></xsl:choose>
                                    </button></td>
                                    <xsl:for-each select="$headings">
                                        <xsl:variable name="heading" select="."/>
                                        <xsl:variable name="cells" select="$record?cells[?key = $heading]"/>
                                        <td><xsl:choose>
                                            <xsl:when test="empty($cells)">∅</xsl:when>
                                            <xsl:otherwise><xsl:for-each select="$cells"><xsl:choose>
                                                <xsl:when test="exists(?subject)"><xsl:apply-templates select="?subject" mode="inspect">
                                                    <xsl:with-param name="key" select="$heading"/><xsl:with-param name="ui" select="$ui"/>
                                                </xsl:apply-templates></xsl:when>
                                                <xsl:when test="?value = ''">""</xsl:when>
                                                <xsl:otherwise><span class="value"><xsl:value-of select="?value"/></span></xsl:otherwise>
                                            </xsl:choose></xsl:for-each></xsl:otherwise>
                                        </xsl:choose></td>
                                    </xsl:for-each>
                                </tr>
                            </xsl:for-each></tbody>
                        </table>
                    </div>
                </xsl:otherwise>
            </xsl:choose>
        </details>
    </xsl:template>
</xsl:stylesheet>
