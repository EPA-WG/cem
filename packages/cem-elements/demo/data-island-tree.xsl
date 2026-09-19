<xsl:stylesheet
    xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
    version="3.0"
>
    <xsl:template name="tree">
        <xsl:param name="source"/>
        <style>
            details { padding: 0 1rem; }
            b { color: green; }
            code { margin-left: 1rem; color: brown; }
        </style>
        <article class="demo-card">
            <h2>XSLT XML payload tree</h2>
            <xsl:for-each select="parse-xml($source)/*">
                <details open="open">
                    <summary>
                        <b><xsl:value-of select="local-name()"/></b>
                        <code>data-root="<xsl:value-of select="@data-root"/>"</code>
                    </summary>
                    <xsl:for-each select="*">
                        <details open="open">
                            <summary>
                                <b><xsl:value-of select="local-name()"/></b>
                                <code>data-level="<xsl:value-of select="@data-level"/>"</code>
                                <code>name="<xsl:value-of select="@name"/>"</code>
                            </summary>
                            <xsl:for-each select="*">
                                <details open="open">
                                    <summary>
                                        <b><xsl:value-of select="local-name()"/></b>
                                        <code>data-level="<xsl:value-of select="@data-level"/>"</code>
                                        <code>code="<xsl:value-of select="@code"/>"</code>
                                    </summary>
                                    <xsl:for-each select="*">
                                        <details open="open">
                                            <summary>
                                                <b><xsl:value-of select="local-name()"/></b>
                                                <code>data-level="<xsl:value-of select="@data-level"/>"</code>
                                            </summary>
                                            <xsl:for-each select="text()">
                                                <p><xsl:value-of select="."/></p>
                                            </xsl:for-each>
                                        </details>
                                    </xsl:for-each>
                                </details>
                            </xsl:for-each>
                        </details>
                    </xsl:for-each>
                </details>
            </xsl:for-each>
        </article>
    </xsl:template>
</xsl:stylesheet>
