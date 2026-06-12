<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:xhtml="http://www.w3.org/1999/xhtml"
                xmlns:dce="urn:schemas-epa-wg:dce" xmlns:exsl="http://exslt.org/common" version="1.0"
                exclude-result-prefixes="exsl">
    <xsl:template match="ignore">
        <xsl:choose>
            <xsl:when test="//attr">
                <xsl:value-of select="//attr"/>
            </xsl:when>
            <xsl:otherwise>
                <xsl:value-of select="def"/>
            </xsl:otherwise>
        </xsl:choose>
        <xsl:value-of select="."/>
    </xsl:template>
    <xsl:template mode="payload" match="attributes">
        <dce-root xmlns="http://www.w3.org/1999/xhtml" xmlns:xhtml="http://www.w3.org/1999/xhtml" data-dce-id="1">
            <http-request xmlns="" method="GET" slice="cem-colors" url="../../../dist/lib/tokens/cem-colors.xhtml"
                          data-dce-id="2"/>
            <xsl:variable xmlns:xsl="http://www.w3.org/1999/XSL/Transform" name="colorsData" select="//cem-colors"/>
            <xsl:variable xmlns:xsl="http://www.w3.org/1999/XSL/Transform" name="cem-color-hue-variant"
                          select="$colorsData//*[@id='cem-color-hue-variant']/following-sibling::xhtml:table[1]/xhtml:tbody"/>
            <xsl:variable xmlns:xsl="http://www.w3.org/1999/XSL/Transform" name="css-body">
                <dce-text xmlns="" data-dce-id="3">

                    :root{
                    /* branded colors: cem-color-[hue]-[variant] */

                </dce-text>
                <span xmlns="" data-dce-id="4">
                    <xsl:for-each select="$cem-color-hue-variant/xhtml:tr">
                        <xsl:value-of select="./*[1]"/> :
                        <xsl:value-of select="./*[4]"/> ; /*
                        <xsl:value-of select="./*[5]"/> ;
                        <xsl:value-of select="./*[6]"/> */
                    </xsl:for-each>
                </span>
                <dce-text xmlns="" data-dce-id="5">

                    /* emotion palette: cem-palette-[emotion]-[shift] */
                </dce-text>
                <xsl:variable name="cem-palette-emotion-heading"
                              select="$colorsData//*[@id='cem-palette-emotion-shift']"/>
                <xsl:variable name="cem-palette-emotion"
                              select="$cem-palette-emotion-heading/following-sibling::xhtml:table[1]/xhtml:tbody"/>
                <dce-text xmlns="" data-dce-id="6">
                    /* light/dark */
                </dce-text>
                <span xmlns="" data-dce-id="7">
                    <xsl:for-each select="$cem-palette-emotion/xhtml:tr">
                        <xsl:variable name="emotion" select="xhtml:td[1]"/>
                        <xsl:variable name="light-val" select="xhtml:td[3]"/>
                        <xsl:variable name="dark-val" select="xhtml:td[4]"/>
                        <span data-dce-id="8">
                            <xsl:choose>
                                <xsl:when test="$dark-val">
                                    <xsl:value-of select="$emotion"/>: light-dark(var( $light-val ), var( $dark-val ) );
                                </xsl:when>
                                <xsl:otherwise>
                                    <xsl:value-of select="$emotion"/>: var( $light-val );
                                </xsl:otherwise>
                            </xsl:choose>
                        </span>
                    </xsl:for-each>
                </span>
                <dce-text xmlns="" data-dce-id="9">
                    /* override for native */
                    .cem-theme-native,[data-theme="cem-theme-native"]{
                </dce-text>
                <span xmlns="" data-dce-id="10">
                    <xsl:for-each select="$cem-palette-emotion/xhtml:tr">
                        <xsl:variable name="emotion" select="xhtml:td[1]"/>
                        <xsl:variable name="light-val" select="xhtml:td[5]"/>
                        <xsl:variable name="dark-val" select="xhtml:td[6]"/>
                        <span data-dce-id="11">
                            <xsl:choose>
                                <xsl:when test="$dark-val">
                                    <xsl:value-of select="$emotion"/>: light-dark(var( $light-val ), var( $dark-val ) );
                                </xsl:when>
                                <xsl:otherwise>
                                    <xsl:value-of select="$emotion"/>: var( $light-val );
                                </xsl:otherwise>
                            </xsl:choose>
                        </span>
                    </xsl:for-each>
                </span>
                <dce-text xmlns="" data-dce-id="12">
                    }
                    }
                </dce-text>
            </xsl:variable>
            <pre xmlns="" class="language-css" data-dce-id="13">
                <code class="language-css" data-dce-id="14">
                    <xsl:value-of select="$css-body"/>
                </code>
            </pre>
            <cem-css-loader xmlns="" value="{$css-body}" data-dce-id="15"/>
            <xsl:for-each xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="$cem-palette-emotion/xhtml:tr">
                <xsl:variable name="emotion" select="xhtml:td[1]"/>
                <xsl:variable name="light-val" select="xhtml:td[5]"/>
                <xsl:variable name="dark-val" select="xhtml:td[6]"/>
                <span xmlns="" data-dce-id="16">
                    <xsl:choose>
                        <xsl:when test="$dark-val">
                            <xsl:value-of select="$emotion"/>: light-dark(var( $light-val ), var( $dark-val ) );
                        </xsl:when>
                        <xsl:otherwise>
                            <xsl:value-of select="$emotion"/>: var( $light-val );
                        </xsl:otherwise>
                    </xsl:choose>
                </span>
            </xsl:for-each>
        </dce-root>
    </xsl:template>
    <xsl:template match="/">
        <xsl:apply-templates mode="payload" select="/datadom/attributes"/>
    </xsl:template>
    <xsl:template name="slot">
        <xsl:param name="slotname"/>
        <xsl:param name="defaultvalue"/>
        <xsl:choose>
            <xsl:when test="//payload/*[@slot=$slotname]">
                <xsl:copy-of select="//payload/*[@slot=$slotname]"/>
            </xsl:when>
            <xsl:otherwise>
                <xsl:copy-of select="$defaultvalue"/>
            </xsl:otherwise>
        </xsl:choose>
    </xsl:template>
    <xsl:variable name="js-injected-body">
        <xsl:call-template name="slot">
            <xsl:with-param name="slotname" select="''"/>
            <xsl:with-param name="defaultvalue"/>
        </xsl:call-template>
    </xsl:variable>
</xsl:stylesheet>