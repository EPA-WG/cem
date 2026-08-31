<xsl:stylesheet
    xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
    xmlns:cem-island="https://cem.dev/ns/runtime/data-island"
    xmlns:cem-payload="https://cem.dev/ns/runtime/instance-payload"
    version="1.0"
>
    <xsl:output method="html" omit-xml-declaration="yes"/>
    <xsl:template match="/">
        <article class="demo-card">
            <h2>XSLT data island tree</h2>
            <xsl:for-each select="/cem-island:context-root/cem-payload:payload/*">
                <details style="padding:0 1rem" open="open">
                    <summary>
                        <b style="color:green"><xsl:value-of select="name()"/></b>
                        <code style="margin-left:1rem;color:brown">data-root="<xsl:value-of select="@data-root"/>"</code>
                    </summary>
                    <xsl:for-each select="*">
                        <details style="padding:0 1rem" open="open">
                            <summary>
                                <b style="color:green"><xsl:value-of select="name()"/></b>
                                <code style="margin-left:1rem;color:brown">data-level="<xsl:value-of select="@data-level"/>"</code>
                                <code style="margin-left:1rem;color:brown">name="<xsl:value-of select="@name"/>"</code>
                            </summary>
                            <xsl:for-each select="*">
                                <details style="padding:0 1rem" open="open">
                                    <summary>
                                        <b style="color:green"><xsl:value-of select="name()"/></b>
                                        <code style="margin-left:1rem;color:brown">data-level="<xsl:value-of select="@data-level"/>"</code>
                                        <code style="margin-left:1rem;color:brown">code="<xsl:value-of select="@code"/>"</code>
                                    </summary>
                                    <xsl:for-each select="*">
                                        <details style="padding:0 1rem" open="open">
                                            <summary>
                                                <b style="color:green"><xsl:value-of select="name()"/></b>
                                                <code style="margin-left:1rem;color:brown">data-level="<xsl:value-of select="@data-level"/>"</code>
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
