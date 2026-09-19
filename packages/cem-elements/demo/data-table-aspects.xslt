<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
    xmlns:g="cem:generic-data" xmlns:j="http://www.w3.org/2005/xpath-functions" version="3.0">
    <xsl:import href="./data-table-view.xslt"/>
    <xsl:template name="viewer-aspects" match="/">
        <xsl:param name="initial" select="string(.)"/>
        <xsl:param name="source" select="$initial"/>
        <xsl:param name="format" select="'json'"/>
        <xsl:param name="column" select="''"/>
        <xsl:param name="direction" select="'ascending'"/>
        <xsl:param name="mode" select="'text'"/>
        <xsl:param name="selected" select="''"/>
        <xsl:param name="aspects" select="true()"/>
        <xsl:param name="ipAddress" select="()"/>
        <xsl:param name="ipAction" select="()"/>
        <label>
            <input type="checkbox" aria-label="Presentation aspects" checked="{$aspects}" slice="aspects"
                slice-event="change" slice-value="$target.checked"/>
            <xsl:text>🧩 Presentation aspects</xsl:text>
        </label>
        <p>Switch off to see the unchanged base view. Notes become a tree;
            the IP-filter record becomes a form. Other arrays keep their tables.</p>
        <xsl:call-template name="viewer">
            <xsl:with-param name="initial" select="$initial"/>
            <xsl:with-param name="source" select="$source"/>
            <xsl:with-param name="format" select="$format"/>
            <xsl:with-param name="column" select="$column"/>
            <xsl:with-param name="direction" select="$direction"/>
            <xsl:with-param name="mode" select="$mode"/>
            <xsl:with-param name="selected" select="$selected"/>
            <xsl:with-param name="presentation" select="map {'aspects':$aspects, 'ipAddress':$ipAddress, 'ipAction':$ipAction}"/>
        </xsl:call-template>
    </xsl:template>

    <xsl:template match="g:array | j:array" mode="inspect" priority="10">
        <xsl:param name="key"/><xsl:param name="ui"/>
        <xsl:choose>
            <xsl:when test="$ui?presentation?aspects and $key = 'notes'">
                <xsl:call-template name="tree">
                    <xsl:with-param name="key" select="$key"/><xsl:with-param name="ui" select="$ui"/>
                </xsl:call-template>
            </xsl:when>
            <xsl:otherwise>
                <xsl:call-template name="table">
                    <xsl:with-param name="rows" select="*"/><xsl:with-param name="key" select="$key"/>
                    <xsl:with-param name="ui" select="$ui"/>
                </xsl:call-template>
            </xsl:otherwise>
        </xsl:choose>
    </xsl:template>

    <xsl:template match="g:object | j:map" mode="inspect" priority="10">
        <xsl:param name="key"/><xsl:param name="ui"/>
        <xsl:choose>
            <xsl:when test="$ui?presentation?aspects and string((g:property[@name='kind']/*, *[@key='kind'])[1]) = 'ip-filter'">
                <xsl:variable name="address" select="if (exists($ui?presentation?ipAddress)) then $ui?presentation?ipAddress else string((g:property[@name='address']/*, *[@key='address'])[1])"/>
                <xsl:variable name="action" select="if (exists($ui?presentation?ipAction)) then $ui?presentation?ipAction else string((g:property[@name='action']/*, *[@key='action'])[1])"/>
                <form method="dialog" aria-label="IP filter" slice="ipFilter">
                    <fieldset>
                        <legend>🛡 IP filter</legend>
                        <label>Address / CIDR
                            <input aria-label="Address / CIDR" name="address" value="{$address}"
                                slice="ipAddress" slice-event="input" slice-value="$target.value"/>
                        </label>
                        <label>Action
                            <select aria-label="Action" name="action" value="{$action}" slice="ipAction" slice-event="change">
                                <option value="allow">✓ Allow</option><option value="deny">⛔ Deny</option>
                            </select>
                        </label>
                        <output aria-live="polite"><xsl:value-of select="$action"/><xsl:text>: </xsl:text><xsl:value-of select="$address"/></output>
                    </fieldset>
                    <p>Preview only; no firewall changes or source write-back.</p>
                </form>
            </xsl:when>
            <xsl:otherwise>
                <xsl:call-template name="tree">
                    <xsl:with-param name="key" select="$key"/><xsl:with-param name="ui" select="$ui"/>
                </xsl:call-template>
            </xsl:otherwise>
        </xsl:choose>
    </xsl:template>
</xsl:stylesheet>
