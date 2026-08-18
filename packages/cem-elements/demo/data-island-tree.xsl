<!doctype html>
<html lang="en">
<body>
<template lang="custom-element-v0">
<article class="demo-card">
    <h2>XSLT data island tree</h2>
    <details style="padding:0 1rem" open="open">
        <summary>
            <b style="color:green">datadom</b>
            <code style="margin-left:1rem;color:brown">title="<xsl:value-of select="$datadom.attributes.title"/>"</code>
            <code style="margin-left:1rem;color:brown">data-demo="<xsl:value-of select="$datadom.attributes.data-demo"/>"</code>
        </summary>
        <xsl:for-each select="datadom.payload.nodes">
            <details style="padding:0 1rem" open="open">
                <summary>
                    <b style="color:green"><xsl:value-of select="$item.tag"/></b>
                    <code style="margin-left:1rem;color:brown">data-root="<xsl:value-of select="$item.attributes.data-root"/>"</code>
                </summary>
                <xsl:for-each select="$item.children">
                    <details style="padding:0 1rem" open="open">
                        <summary>
                            <b style="color:green"><xsl:value-of select="$item.tag"/></b>
                            <code style="margin-left:1rem;color:brown">data-level="<xsl:value-of select="$item.attributes.data-level"/>"</code>
                            <code style="margin-left:1rem;color:brown">name="<xsl:value-of select="$item.attributes.name"/>"</code>
                        </summary>
                        <xsl:for-each select="$item.children">
                            <details style="padding:0 1rem" open="open">
                                <summary>
                                    <b style="color:green"><xsl:value-of select="$item.tag"/></b>
                                    <code style="margin-left:1rem;color:brown">data-level="<xsl:value-of select="$item.attributes.data-level"/>"</code>
                                    <code style="margin-left:1rem;color:brown">code="<xsl:value-of select="$item.attributes.code"/>"</code>
                                </summary>
                                <xsl:for-each select="$item.children">
                                    <details style="padding:0 1rem" open="open">
                                        <summary>
                                            <b style="color:green"><xsl:value-of select="$item.tag"/></b>
                                            <code style="margin-left:1rem;color:brown">data-level="<xsl:value-of select="$item.attributes.data-level"/>"</code>
                                        </summary>
                                        <xsl:for-each select="$item.children">
                                            <p><xsl:value-of select="$item.text"/></p>
                                        </xsl:for-each>
                                    </details>
                                </xsl:for-each>
                            </details>
                        </xsl:for-each>
                    </details>
                </xsl:for-each>
            </details>
        </xsl:for-each>
    </details>
</article>
</template>
</body>
</html>
