/* Copyright (C) Pometry Ltd - All Rights Reserved.
 *
 * This file is proprietary and confidential. Unauthorised
 * copying of this file, via any medium is strictly prohibited.
 *
 */

package com.raphtory.arrowcore.implementation;

import com.raphtory.arrowcore.util.LRUListItem;
import org.apache.arrow.vector.VectorSchemaRoot;
import org.apache.arrow.vector.ipc.ArrowFileReader;
import org.apache.arrow.vector.ipc.ArrowFileWriter;
import org.apache.arrow.vector.types.pojo.Schema;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;

/**
 * This class manages access to a single Arrow property file, loading, saving,
 * managing history and accessing values.
 *<p>
 * Properties are stored in the order they're added/inserted, however the linked
 * list of properties for a particular vertex is always maintained in time order.
 *<p>
 * It is associated with/owned by a vertex partition.
 */
public class VertexPropertyPartition implements LRUListItem<VertexPropertyPartition> {
    protected final int _partitionId;
    protected final VertexPartitionManager _apm;
    protected final int _propertyId;

    protected VectorSchemaRoot _rootRO;
    protected VersionedPropertyStore _accessor;
    protected ArrowFileReader _reader;
    protected boolean _modified = false;
    protected PropertyStore _store;
    protected VersionedProperty _property;

    private VertexPropertyPartition _prev = null;
    private VertexPropertyPartition _next = null;


    /**
     * Instantiates an instance looking after a particular property for a vertex.
     *
     * @param apm the vertex partition manager to use
     * @param partitionId the partition id of the owning vertex partition
     * @param propertyId the vertex property id in question
     * @param property the property field in question
     */
    public VertexPropertyPartition(VertexPartitionManager apm, int partitionId, int propertyId, VersionedProperty property) {
        _apm = apm;
        _partitionId = partitionId;
        _propertyId = propertyId;
        _property = property;

        _rootRO = null;
        _store = new PropertyStore();
        _accessor = _apm._raphtoryPartition.createSchemaPropertyAccessor(_rootRO, _property);
    }


    /**
     * Initializes this instance for an empty file.
     */
    public void initialize() {
        Schema arrowSchema = _apm._raphtoryPartition.getVertexPropertySchema(_propertyId);

        _rootRO = VectorSchemaRoot.create(arrowSchema, _apm.getAllocator());
        _rootRO.setRowCount(_apm.PARTITION_SIZE);

        _store.init(_partitionId, _rootRO, _accessor);
    }


    /**
     * @return the ArrowPropertySchema for this instance
     */
    public PropertyStore getSchema() {
        return _store;
    }


    /**
     * @return the partitionId for this instance
     */
    public int getPartitionId() {
        return _partitionId;
    }


    /**
     * Add a property value, including history
     *
     * @param prevPtr the head of the the list of previous values (history)
     * @param p the property accessor containing the actual values
     *
     * @return the row number that the property was stored at
     */
    public int addProperty(int prevPtr, VersionedEntityPropertyAccessor p) {
        _modified = true;
        int row = _store.addProperty(p.getLocalId(), p.getInitialValue(), p.getCreationTime(), prevPtr, p);
        return row;
    }


    /**
     * Retrieves a property value from a particular row
     *
     * @param row the row to read
     * @param ea the destination entity property accessor to update with the value
     */
    public void retrieveProperty(int row, VersionedEntityPropertyAccessor ea) { // MT
        if (_rootRO == null) {
            return;
        }

        _store.loadProperty(row, ea);
    }


    /**
     * LRUList implementation - set the next ptr
     *
     * @param p the next pointer
     */
    @Override
    public void setNext(VertexPropertyPartition p) {
        _next = p;
    }


    /**
     * LRUList implementation - set the prev ptr
     *
     * @param p the prev pointer
     */
    @Override
    public void setPrev(VertexPropertyPartition p) {
        _prev = p;
    }


    /**
     * LRUList implementation - get the next pointer
     *
     * @return the next pointer in this node
     */
    @Override
    public VertexPropertyPartition getNext() {
        return _next;
    }


    /**
     * LRUList implementation - get the prev pointer
     *
     * @return the prev pointer in this node
     */
    @Override
    public VertexPropertyPartition getPrev() {
        return _prev;
    }


    /**
     * Closes this instance, releasing all resources.
     */
    public void close() {
        clearReader();
    }


    /**
     * Saves this property partition to a disk file.
     *
     * @return true if successfully saved, false otherwise
     */
    public boolean saveToFile() {
        try {
            if (_modified) {
                _rootRO.syncSchema();
                _rootRO.setRowCount(_store._maxRow);

                File outFile = _apm.getVertexPropertyFile(_partitionId, _propertyId);
                ArrowFileWriter writer = new ArrowFileWriter(_rootRO, null, new FileOutputStream(outFile).getChannel());
                writer.start();
                writer.writeBatch();
                writer.end();

                writer.close();
            }

            _modified = false;
            return true;
        }
        catch (Exception e) {
            System.out.println("Exception: " + e);
            e.printStackTrace(System.err);
            return false;
        }
    }


    /**
     * Loads this property partition from a disk file
     *
     * @return true if successfully loaded, false if not
     */
    public boolean loadFromFile() {
        File inFile = _apm.getVertexPropertyFile(_partitionId, _propertyId);
        if (!inFile.exists()) {
            //System.out.println("Not loading vertex partition: " + _partitionId);
            return false;
        }

        //System.out.println("VERTEX LOADING PARTITION: " + _partitionId);
        long then = System.currentTimeMillis();

        try {
            clearReader();

            //CallLoggingFileChannel proxy = new CallLoggingFileChannel(new FileInputStream(inFile).getChannel());
            //_reader = new ArrowFileReader(proxy, _apm.getAllocator(), _apm.getCompressionFactory());

            _reader = new ArrowFileReader(new FileInputStream(inFile).getChannel(), _apm.getAllocator(), _apm.getCompressionFactory());

            _reader.loadNextBatch();
            _rootRO = _reader.getVectorSchemaRoot();

            _rootRO.syncSchema();

            _store.init(_partitionId, _rootRO, _accessor);

            _modified = false;
            _store._maxRow = _rootRO.getRowCount();

            return true;
        }
        catch (Exception e) {
            System.err.println("Exception: " + e);
            e.printStackTrace(System.err);
            return false;
        }
        finally {
            long now = System.currentTimeMillis();
            //System.out.println("VERTEXLOAD: PID=" + _partitionId + " " + (now-then) + "mS");
        }
    }


    /**
     * Clears the Arrow components, releasing resources
     */
    private void clearReader() {
        try {
            if (_store != null) {
                _store.init(_partitionId, null, _accessor);
            }

            if (_rootRO != null) {
                _rootRO.clear();
                _rootRO.close();
                _rootRO = null;
            }

            if (_reader != null) {
                _reader.close();
                _reader = null;
            }
        }
        catch (Exception e) {
            System.err.println("Exception: " + e);
            e.printStackTrace(System.err);
        }
    }


    /**
     * Returns the creation-time at the given row
     *
     * @param row the row in question
     *
     * @return the creation time stored at that row
     */
    protected long getCreationTime(int row) {
        return _store._creationTimes.get(row);
    }


    /**
     * Returns the next row where the next item in the history for this
     * property is stored.
     *
     * @param row the current point in the history for this property
     *
     * @return the next row containing relevant history
     */
    protected int getNextRowInList(int row) {
        return _store._prevPtrs.get(row);
    }


    /**
     * Inserts a property in time order into the store for a vertex.
     *
     * @param headRow the initial head of the history list for this property for this vertex
     * @param creationTime the creation time of this value
     * @param efa the actual value
     *
     * @return the row number where this item was stored
     */
    public int insertProperty(int headRow, long creationTime, VersionedEntityPropertyAccessor efa) {
        int prev = -1;
        int next = headRow;

        while (next!=-1 && getCreationTime(next) > creationTime) {
            prev = next;
            next = getNextRowInList(next);
        }

        int row = addProperty(next, efa);
        if (prev!=-1) {
            _store._prevPtrs.set(prev, row);
        }

        return row;
    }
}